// Synthetic desktop peer for actual native-host relay tests; guest only.
#include "../../src-tauri/native/browser_bridge.hpp"
#include <cstdio>
using namespace grammar;
int wmain(int argc,wchar_t** argv) {
    WCHAR user[128]{};DWORD n=128;if(argc!=4 || !GetUserNameW(user,&n) || _wcsicmp(user,L"WDAGUtilityAccount"))return 3;
    const DWORD pid=wcstoul(argv[1],nullptr,10);const int scenario=_wtoi(argv[3]);
    ChannelHandle file(CreateFileW(argv[2],GENERIC_READ,FILE_SHARE_READ,nullptr,OPEN_EXISTING,FILE_ATTRIBUTE_NORMAL,nullptr));
    ApplyImageDigest hash{};auto deadline=GetTickCount64()+3000;
    if(file.get()==INVALID_HANDLE_VALUE || !apply_image_digest(file.get(),hash,deadline))return 4;
    BrowserBridgeClient client;if(!client.connect(verify_apply_peer(pid,argv[2],hash,deadline),deadline))return 5;
    TargetReply reply;std::wstring text;bool captured=client.request(ChannelCommand::capture,L"",reply,text);
    if(scenario==1 || scenario==3 || scenario==4){printf("capture_rejected=%d\n",!captured);return captured?6:0;}
    if(!captured || reply.status!=2 || text!=L"\xC120\xD0DD \xD83D\xDE00")return 7;
    bool applied=client.request(ChannelCommand::apply,L"\xAC80\xD1A0 \xD83D\xDE42",reply,text);
    bool correct=scenario==2?(!applied && reply.status==4):(applied && reply.status==3);
    bool replay=!client.request(ChannelCommand::apply,L"replay",reply,text);
    printf("captured=1 outcome_correct=%d replay_refused=%d\n",correct,replay);return correct&&replay?0:8;
}

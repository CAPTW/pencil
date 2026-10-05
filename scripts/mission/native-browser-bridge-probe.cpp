// Synthetic IPC qualification. No document/editor/clipboard/Provider operation.
#include "../../src-tauri/native/browser_bridge.hpp"
#include <thread>
#include <cstdio>
using namespace grammar;
static std::wstring image() {WCHAR path[32768]{};DWORD n=GetModuleFileNameW(nullptr,path,32768);return n && n<32768?std::wstring(path,n):L"";}
static ApplyImageDigest digest() {
    ChannelHandle file(CreateFileW(image().c_str(),GENERIC_READ,FILE_SHARE_READ,nullptr,OPEN_EXISTING,FILE_ATTRIBUTE_NORMAL,nullptr));
    ApplyImageDigest value{};if(!apply_image_digest(file.get(),value,GetTickCount64()+3000))throw 1;return value;
}
static int serve(int scenario) {
    auto hash=digest();if(scenario==1)hash[0]^=1;
    BrowserBridgeServer server(image(),hash);ChannelRequest request;unsigned calls=0;
    const auto eventName=L"Local\\Grammar.BridgeProbe."+std::to_wstring(GetCurrentProcessId());
    ChannelHandle capturedEvent(CreateEventW(nullptr,TRUE,FALSE,eventName.c_str()));
    if(!capturedEvent.get())return 25;
    const auto deadline=GetTickCount64()+(scenario==11?40000:12000);
    while(GetTickCount64()<deadline) {
        auto state=server.poll(request);
        if(state==BrowserPoll::closed) {
            unsigned expected=(scenario==0 || scenario==9 || scenario==10)?2:(scenario==2 || scenario==1 || scenario==3 || scenario==4 || scenario==5 || scenario==6 || scenario==7)?0:1;
            return calls==expected?0:20;
        }
        if(state==BrowserPoll::request) {
            ++calls;
            if(request.header.command==ChannelCommand::capture) {
                if(scenario==8 || scenario==12){SetEvent(capturedEvent.get());continue;} // Parent cancels while awaiting source.
                if(!server.respond(2,L"\xC120\xD0DD \xD83D\xDE00"))return 21;
            } else {
                if(scenario==9){server.close();return 0;} // Lost physical outcome, never report rejected.
                std::wstring value(request.body.size()/2,L'\0');memcpy(value.data(),request.body.data(),request.body.size());
                if(value!=L"\xAC80\xD1A0 \xD83D\xDE42")return 22;
                if(!server.respond(scenario==10?4:3))return 23;
            }
        }
        Sleep(1);
    }
    return 24;
}
int wmain(int argc,wchar_t** argv) try {
    WCHAR user[128]{};DWORD size=128;
    if(!GetUserNameW(user,&size) || _wcsicmp(user,L"WDAGUtilityAccount"))return 3;
    if(argc==3 && !wcscmp(argv[1],L"--server"))return serve(_wtoi(argv[2]));
    unsigned failures=0,cases=0;
    for(int scenario=0;scenario<=12;++scenario) {
#ifdef GRAMMAR_ABI_PROBE
        if(scenario!=0 && scenario!=8 && scenario!=9 && scenario!=10 && scenario!=11 && scenario!=12)continue;
#endif
        ++cases;
        std::wstring command=L"\""+image()+L"\" --server "+std::to_wstring(scenario);
        STARTUPINFOW startup{};startup.cb=sizeof(startup);PROCESS_INFORMATION child{};
        if(!CreateProcessW(image().c_str(),command.data(),nullptr,nullptr,FALSE,CREATE_NO_WINDOW,nullptr,nullptr,&startup,&child))return 4;
        ChannelHandle process(child.hProcess),thread(child.hThread);bool pass=false;
        const auto deadline=GetTickCount64()+3000;auto hash=digest();
        if(scenario==2)hash[0]^=1;
        auto peer=verify_apply_peer(child.dwProcessId,image(),hash,deadline);
        if(scenario==2) {
            pass=!peer;
            // The rejected client never connects; explicitly give the owned
            // server a connection it will reject so its bounded loop can end.
            ChannelHandle raw;
            do {raw.reset(CreateFileW(browser_bridge_name(child.dwProcessId).c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,0,nullptr));if(raw.get()!=INVALID_HANDLE_VALUE)break;Sleep(1);}while(GetTickCount64()<deadline);
        } else if(scenario>=3 && scenario<=7) {
            ChannelHandle raw;
            do {raw.reset(CreateFileW(browser_bridge_name(child.dwProcessId).c_str(),GENERIC_READ|GENERIC_WRITE,0,nullptr,OPEN_EXISTING,SECURITY_SQOS_PRESENT|SECURITY_IDENTIFICATION,nullptr));if(raw.get()!=INVALID_HANDLE_VALUE)break;Sleep(1);}while(GetTickCount64()<deadline);
            DWORD mode=PIPE_NOWAIT;TargetHello hello;
            bool ready=peer && raw.get()!=INVALID_HANDLE_VALUE && SetNamedPipeHandleState(raw.get(),&mode,nullptr,nullptr) && channel_read_exact(raw.get(),&hello,sizeof(hello),deadline);
            ChannelHeader bad;bad.capability=hello.capability;
            if(scenario==3)bad.capability[0]^=1;
            if(scenario==4)bad.sequence=2;
            if(scenario==5){bad.command=ChannelCommand::apply;bad.capture_id=1;bad.payload_bytes=2;}
            if(scenario==6){bad.command=ChannelCommand::apply;bad.capture_id=1;bad.payload_bytes=(8192+1)*2;}
            if(scenario==7)bad.version=1;
            const wchar_t nul=0;std::wstring huge(8193,L'x');
            pass=ready && target_write(raw.get(),&bad,sizeof(bad),deadline);
            if(scenario==5)pass=pass && target_write(raw.get(),&nul,2,deadline);
            if(scenario==6)(void)target_write(raw.get(),huge.data(),static_cast<DWORD>(huge.size()*2),deadline);
            SecureZeroMemory(hello.capability.data(),hello.capability.size());
            // Keep the client connected until the server actually rejects it;
            // closing first would prove only disconnect, not header admission.
            pass=pass && WaitForSingleObject(process.get(),4000)==WAIT_OBJECT_0;
        } else {
            BrowserBridgeClient client;bool connected=client.connect(std::move(peer),deadline);
            if(scenario==1)pass=!connected;
            else if(connected) {
                TargetReply reply;std::wstring text;std::thread cancellation;
                bool cancellationSignalled=false;
                if(scenario==8)cancellation=std::thread([&]{
                    const auto name=L"Local\\Grammar.BridgeProbe."+std::to_wstring(child.dwProcessId);
                    ChannelHandle event(OpenEventW(SYNCHRONIZE,FALSE,name.c_str()));
                    if(event.get() && WaitForSingleObject(event.get(),2500)==WAIT_OBJECT_0){cancellationSignalled=true;client.cancel();}
                });
                bool captured=client.request(ChannelCommand::capture,L"",reply,text);
                if(cancellation.joinable())cancellation.join();
                if(scenario==8 || scenario==12)pass=!captured && text.empty() && (scenario!=8 || cancellationSignalled);
                else if(scenario==11){pass=captured;Sleep(35100);pass=pass && !client.request(ChannelCommand::apply,L"expired",reply,text);}
                else {
                    pass=captured && reply.status==2 && reply.capture_id==1 && text==L"\xC120\xD0DD \xD83D\xDE00";
                    bool applied=client.request(ChannelCommand::apply,L"\xAC80\xD1A0 \xD83D\xDE42",reply,text);
                    pass=pass && (scenario==9?(!applied && reply.status==4):(applied && reply.status==(scenario==10?4u:3u)));
                    // A consumed Apply cannot be replayed, even after a lost reply.
                    pass=pass && !client.request(ChannelCommand::apply,L"again",reply,text);
                }
            }
        }
        bool exited=WaitForSingleObject(process.get(),5000)==WAIT_OBJECT_0;DWORD code=STILL_ACTIVE;GetExitCodeProcess(process.get(),&code);
        pass=pass && exited && code==0;
        printf("case=%d pass=%d child_exited=%d child_exit=%lu\n",scenario,pass,exited,code);fflush(stdout);
        if(!pass)++failures;if(!exited)return 8; // No more children after unconfirmed cleanup.
    }
    printf("cases=%u failures=%u\n",cases,failures);return failures?1:0;
} catch(...){return 9;}

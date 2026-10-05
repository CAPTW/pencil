// Synthetic guest-only caller for the real Writer client ABI.
#include <windows.h>
#include <cstdint>
#include <cstdio>
#include <cwchar>
#include <string>
int wmain(int argc,wchar_t** argv){
    wchar_t user[256]{};DWORD count=256;
    if(argc!=3||!GetUserNameW(user,&count)||wcscmp(user,L"WDAGUtilityAccount"))return 90;
    const std::wstring library=argv[1];
    if(library.rfind(L"C:\\GrammarPayload\\writer-native\\",0)!=0)return 91;
    wchar_t* end=nullptr;const auto raw=_wcstoui64(argv[2],&end,10);
    if(!raw||!end||*end)return 92;
    const auto module=LoadLibraryExW(library.c_str(),nullptr,LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR|LOAD_LIBRARY_SEARCH_SYSTEM32);
    if(!module)return 93;
    using Create=uint32_t(WINAPI*)(HWND,uint64_t*);
    using Capture=uint32_t(WINAPI*)(uint64_t,wchar_t*,uint32_t,uint32_t*);
    using Apply=uint32_t(WINAPI*)(uint64_t,const wchar_t*,uint32_t,uint32_t*);
    using Close=uint32_t(WINAPI*)(uint64_t,uint32_t);
    auto create=reinterpret_cast<Create>(GetProcAddress(module,"GrammarWriterClientCreate"));
    auto capture=reinterpret_cast<Capture>(GetProcAddress(module,"GrammarWriterClientCapture"));
    auto apply=reinterpret_cast<Apply>(GetProcAddress(module,"GrammarWriterClientApply"));
    auto close=reinterpret_cast<Close>(GetProcAddress(module,"GrammarWriterClientClose"));
    if(!create||!capture||!apply||!close)return 94;
    uint64_t cookie=0;wchar_t text[8193]{};uint32_t units=0,outcome=4,applyCode=99;
    const auto created=create(reinterpret_cast<HWND>(raw),&cookie);
    const auto captured=created?99:capture(cookie,text,8193,&units);
    const bool matched=!captured&&std::wstring(text,units)==L"Synthetic";
    SecureZeroMemory(text,sizeof(text));
    const std::wstring replacement=L"Reviewed \ud55c\uad6d\uc5b4 \U0001f600";
    if(matched)applyCode=apply(cookie,replacement.c_str(),static_cast<uint32_t>(replacement.size()),&outcome);
    const auto closed=cookie?close(cookie,1000):0;
    std::printf("{\"created\":%u,\"captured\":%u,\"sourceMatched\":%s,\"applyCode\":%u,\"outcome\":%u,\"closed\":%u}\n",
        created,captured,matched?"true":"false",applyCode,outcome,closed);
    FreeLibrary(module);
    return 0;
}

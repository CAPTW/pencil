// Native host C ABI. Installation-bound peer trust is compiled into the same
// target-apply.dll as the desktop target service. No caller-supplied trust path.
#include "browser_bridge.hpp"
#include <mutex>
#ifndef GRAMMAR_TRUSTED_APP_SHA256
#error Supply the trusted broker build header.
#endif
namespace {
using namespace grammar;
std::mutex browser_mutex;
std::unique_ptr<BrowserBridgeServer> browser_server;
uint64_t browser_cookie=0,next_browser_cookie=1;
std::wstring browser_trusted_app() {
    HMODULE module=nullptr;
    if(!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS|GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        reinterpret_cast<LPCWSTR>(&browser_trusted_app),&module))return {};
    WCHAR path[32768]{};DWORD length=GetModuleFileNameW(module,path,32768);
    if(!length || length>=32768)return {};
    const std::wstring value(path,length);const auto slash=value.find_last_of(L"\\/");
    return slash==std::wstring::npos?L"":value.substr(0,slash+1)+GRAMMAR_TRUSTED_APP_NAME;
}
bool browser_trusted_hash(ApplyImageDigest& hash) {
    const std::string text=GRAMMAR_TRUSTED_APP_SHA256;if(text.size()!=64)return false;
    auto nibble=[](char c){return c>='0'&&c<='9'?c-'0':c>='a'&&c<='f'?c-'a'+10:-1;};
    for(size_t i=0;i<hash.size();++i){int a=nibble(text[2*i]),b=nibble(text[2*i+1]);if(a<0||b<0)return false;hash[i]=static_cast<BYTE>((a<<4)|b);}
    return true;
}
}
// Codes:0 success,1 unknown owner,2 busy/owner retained,3 invalid input,
//4 unavailable,5 internal failure. Poll states:0 idle,1 command,2 closed.
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarBrowserServerVersion() noexcept {return 1;}
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarBrowserServerCreate(uint64_t* cookie) noexcept try {
    if(!cookie)return 3;*cookie=0;
    std::unique_lock<std::mutex> lock(browser_mutex,std::try_to_lock);if(!lock.owns_lock() || browser_server)return 2;
    ApplyImageDigest hash{};auto app=browser_trusted_app();
    if(app.empty() || !browser_trusted_hash(hash) || next_browser_cookie==UINT64_MAX)return 4;
    auto server=std::make_unique<BrowserBridgeServer>(app,hash);if(!server->available())return 4;
    browser_cookie=next_browser_cookie++;browser_server=std::move(server);*cookie=browser_cookie;return 0;
} catch(...){return 5;}
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarBrowserServerPoll(uint64_t cookie,uint32_t* state,
    uint32_t* command,uint64_t* capture,WCHAR* text,uint32_t capacity,uint32_t* units) noexcept try {
    if(!state || !command || !capture || !text || capacity<browser_bridge_units+1 || !units)return 3;
    *state=2;*command=0;*capture=0;*units=0;text[0]=0;
    std::unique_lock<std::mutex> lock(browser_mutex,std::try_to_lock);if(!lock.owns_lock())return 2;
    if(!browser_server || cookie!=browser_cookie)return 1;
    ChannelRequest request;const auto result=browser_server->poll(request);
    if(result==BrowserPoll::idle){*state=0;return 0;}
    if(result==BrowserPoll::closed)return 0;
    *state=1;*command=static_cast<uint32_t>(request.header.command);*capture=request.header.capture_id;
    *units=static_cast<uint32_t>(request.body.size()/sizeof(wchar_t));
    if(*units)memcpy(text,request.body.data(),request.body.size());text[*units]=0;return 0;
} catch(...){return 5;}
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarBrowserServerRespond(uint64_t cookie,uint32_t status,
    const WCHAR* text,uint32_t units) noexcept try {
    if(units>browser_bridge_units || (!text && units) ||
        (status==2?units==0:(units!=0 || (status!=0 && status!=3 && status!=4))))return 3;
    std::unique_lock<std::mutex> lock(browser_mutex,std::try_to_lock);if(!lock.owns_lock())return 2;
    if(!browser_server || cookie!=browser_cookie)return 1;
    std::wstring source=units?std::wstring(text,units):L"";
    const bool sent=browser_server->respond(status,source);
    if(!source.empty())SecureZeroMemory(source.data(),source.size()*sizeof(wchar_t));return sent?0:4;
} catch(...){return 5;}
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarBrowserServerClose(uint64_t cookie) noexcept try {
    std::unique_lock<std::mutex> lock(browser_mutex,std::try_to_lock);if(!lock.owns_lock())return 2;
    if(!browser_server || cookie!=browser_cookie)return 1;
    browser_server->close();browser_server.reset();browser_cookie=0;return 0;
} catch(...){return 5;}

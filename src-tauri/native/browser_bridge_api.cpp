// Native host C ABI. Installation-bound peer trust is compiled into the same
// target-apply.dll as the desktop target service. No caller-supplied trust path.
#include "browser_bridge.hpp"
#include "browser_route.hpp"
#include <mutex>
#include <chrono>
#ifndef GRAMMAR_TRUSTED_APP_SHA256
#error Supply the trusted broker build header.
#endif
#ifndef GRAMMAR_TRUSTED_HOST_SHA256
#define GRAMMAR_TRUSTED_HOST_SHA256 ""
#endif
namespace {
using namespace grammar;
std::mutex browser_mutex;
std::unique_ptr<BrowserBridgeServer> browser_server;
uint64_t browser_cookie=0,next_browser_cookie=1;
std::wstring browser_sibling(const wchar_t* name) {
    HMODULE module=nullptr;
    if(!GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS|GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        reinterpret_cast<LPCWSTR>(&browser_sibling),&module))return {};
    WCHAR path[32768]{};DWORD length=GetModuleFileNameW(module,path,32768);
    if(!length || length>=32768)return {};
    const std::wstring value(path,length);const auto slash=value.find_last_of(L"\\/");
    return slash==std::wstring::npos?L"":value.substr(0,slash+1)+name;
}
bool browser_digest(const std::string& text,ApplyImageDigest& hash) {
    if(text.size()!=64)return false;
    auto nibble=[](char c){return c>='0'&&c<='9'?c-'0':c>='a'&&c<='f'?c-'a'+10:-1;};
    for(size_t i=0;i<hash.size();++i){int a=nibble(text[2*i]),b=nibble(text[2*i+1]);if(a<0||b<0)return false;hash[i]=static_cast<BYTE>((a<<4)|b);}
    return true;
}
}
// Codes:0 success,1 unknown owner,2 busy/owner retained,3 invalid input,
//4 unavailable,5 internal failure. Poll states:0 idle,1 command,2 closed.
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarBrowserServerVersion() noexcept {return 2;}
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarBrowserServerCreateV2(uint64_t* cookie,uint64_t* clock_ticks) noexcept try {
    if(!cookie || !clock_ticks)return 3;*cookie=0;*clock_ticks=0;
    // Cross a native tick boundary after browser receipt, so quantization cannot
    // place the native anchor before the browser's earlier send timestamp.
    const auto initial=GetTickCount64();auto tick=initial;
    const auto limit=std::chrono::steady_clock::now()+std::chrono::milliseconds(100);
    while(tick==initial){if(std::chrono::steady_clock::now()>=limit)return 4;Sleep(1);tick=GetTickCount64();}
    *clock_ticks=tick;
    std::unique_lock<std::mutex> lock(browser_mutex,std::try_to_lock);if(!lock.owns_lock() || browser_server)return 2;
    ApplyImageDigest hash{};auto app=browser_sibling(GRAMMAR_TRUSTED_APP_NAME);
    if(app.empty() || !browser_digest(GRAMMAR_TRUSTED_APP_SHA256,hash) || next_browser_cookie==UINT64_MAX)return 4;
    auto server=std::make_unique<BrowserBridgeServer>(app,hash);if(!server->available())return 4;
    browser_cookie=next_browser_cookie++;browser_server=std::move(server);*cookie=browser_cookie;return 0;
} catch(...){return 5;}
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarBrowserServerPollV2(uint64_t cookie,uint32_t* state,
    uint32_t* command,uint64_t* capture,WCHAR* text,uint32_t capacity,uint32_t* units,uint64_t* deadline_ticks) noexcept try {
    if(!state || !command || !capture || !text || capacity<browser_bridge_units+1 || !units || !deadline_ticks)return 3;
    *state=2;*command=0;*capture=0;*units=0;*deadline_ticks=0;text[0]=0;
    std::unique_lock<std::mutex> lock(browser_mutex,std::try_to_lock);if(!lock.owns_lock())return 2;
    if(!browser_server || cookie!=browser_cookie)return 1;
    ChannelRequest request;const auto result=browser_server->poll(request);
    if(result==BrowserPoll::idle){*state=0;return 0;}
    if(result==BrowserPoll::closed)return 0;
    *deadline_ticks=browser_server->operation_deadline();
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

namespace {
struct BrowserClientSlot {
    const uint64_t cookie;
    const HWND window;
    std::mutex operation;
    std::atomic<bool> cancelled{false};
    BrowserBridgeClient client;
    std::unique_ptr<BrowserProcessRoute> route;
    bool opened=false,captured=false;
    BrowserClientSlot(uint64_t id,HWND target):cookie(id),window(target){}
};
std::mutex browser_client_registry;
std::shared_ptr<BrowserClientSlot> browser_client;
uint64_t next_browser_client=1;
std::shared_ptr<BrowserClientSlot> browser_client_lookup(uint64_t cookie) {
    std::lock_guard<std::mutex> lock(browser_client_registry);
    return browser_client && browser_client->cookie==cookie?browser_client:nullptr;
}
std::unique_ptr<BrowserProcessRoute> browser_find_route(const BrowserClientSlot& slot,
    const std::wstring& path,const ApplyImageDigest& hash,ULONGLONG deadline) {
    // A just-closed prior review may be rearmed by the opted-in field heartbeat.
    // Wait only inside the original capture budget, never start a new budget.
    while(!slot.cancelled && GetTickCount64()<deadline) {
    if(GetForegroundWindow()!=slot.window)return {};
    ChannelHandle snapshot(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS,0));
    PROCESSENTRY32W entry{};entry.dwSize=sizeof(entry);
    if(snapshot.get()==INVALID_HANDLE_VALUE || !Process32FirstW(snapshot.get(),&entry))return {};
    std::unique_ptr<BrowserProcessRoute> route;unsigned visited=0;
    do {
        if(++visited>65536 || slot.cancelled || GetTickCount64()>=deadline)return {};
        if(_wcsicmp(entry.szExeFile,L"grammar-chromium-host.exe"))continue;
        if(!WaitNamedPipeW(browser_bridge_name(entry.th32ProcessID).c_str(),1))continue;
        auto candidate=verify_browser_route(entry.th32ProcessID,slot.window,path,hash,deadline);
        if(candidate){if(route)return {};route=std::move(candidate);}
    }while(Process32NextW(snapshot.get(),&entry));
    if(route)return route;
    Sleep(20);
    }
    return {};
}
}
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarBrowserClientCreate(HWND window,uint64_t* cookie) noexcept try {
    if(!window || !cookie)return 3;*cookie=0;
    ApplyImageDigest hash{};if(!browser_digest(GRAMMAR_TRUSTED_HOST_SHA256,hash))return 4;
    std::lock_guard<std::mutex> lock(browser_client_registry);
    if(browser_client)return 2;if(next_browser_client==UINT64_MAX)return 5;
    browser_client=std::make_shared<BrowserClientSlot>(next_browser_client++,window);
    *cookie=browser_client->cookie;return 0;
} catch(...){return 5;}
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarBrowserClientCapture(uint64_t cookie,WCHAR* text,uint32_t capacity,uint32_t* units) noexcept try {
    if(!text || capacity<browser_bridge_units+1 || !units)return 3;*units=0;text[0]=0;
    auto slot=browser_client_lookup(cookie);if(!slot)return 1;
    std::unique_lock<std::mutex> lock(slot->operation,std::try_to_lock);if(!lock.owns_lock())return 2;
    if(slot->opened || slot->cancelled)return 4;slot->opened=true;
    const auto deadline=GetTickCount64()+3000;
    const auto path=browser_sibling(L"grammar-chromium-host.exe");ApplyImageDigest hash{};
    if(path.empty() || !browser_digest(GRAMMAR_TRUSTED_HOST_SHA256,hash))return 4;
    slot->route=browser_find_route(*slot,path,hash,deadline);
    if(!slot->route || slot->cancelled || !slot->route->foreground())return 4;
    if(!slot->client.connect(verify_apply_peer(slot->route->host_pid(),path,hash,deadline),deadline))return 4;
    TargetReply reply;std::wstring source;
    const bool ok=slot->client.request(ChannelCommand::capture,L"",reply,source,deadline) && reply.status==2 &&
        !slot->cancelled && slot->route->foreground() && GetTickCount64()<deadline;
    if(ok){slot->captured=true;*units=static_cast<uint32_t>(source.size());memcpy(text,source.data(),source.size()*sizeof(wchar_t));text[*units]=0;}
    if(!source.empty())SecureZeroMemory(source.data(),source.size()*sizeof(wchar_t));
    if(!ok)slot->client.disconnect();return ok?0:4;
} catch(...){return 5;}
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarBrowserClientApply(uint64_t cookie,const WCHAR* text,uint32_t units,uint32_t* outcome) noexcept try {
    if(!text || !units || units>browser_bridge_units || !outcome)return 3;*outcome=4;
    auto slot=browser_client_lookup(cookie);if(!slot)return 1;
    std::unique_lock<std::mutex> lock(slot->operation,std::try_to_lock);if(!lock.owns_lock())return 2;
    if(!slot->captured || slot->cancelled || !slot->route || !slot->route->alive()){*outcome=0;return 0;}
    slot->captured=false;
    TargetReply reply;std::wstring ignored,replacement(text,units);
    const bool ok=slot->client.request(ChannelCommand::apply,replacement,reply,ignored);
    SecureZeroMemory(replacement.data(),replacement.size()*sizeof(wchar_t));
    if(ok)*outcome=reply.status;return ok?0:4;
} catch(...){return 5;}
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarBrowserClientCancel(uint64_t cookie) noexcept try {
    auto slot=browser_client_lookup(cookie);if(!slot)return 1;
    slot->cancelled=true;slot->client.cancel();
    std::unique_lock<std::mutex> lock(slot->operation,std::try_to_lock);
    if(lock.owns_lock()){slot->captured=false;slot->client.disconnect();slot->route.reset();}return 0;
} catch(...){return 5;}
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarBrowserClientClose(uint64_t cookie,uint32_t milliseconds) noexcept try {
    if(milliseconds>8000)return 3;
    auto slot=browser_client_lookup(cookie);if(!slot)return 1;
    slot->cancelled=true;slot->client.cancel();
    std::unique_lock<std::mutex> lock(slot->operation,std::try_to_lock);if(!lock.owns_lock())return 2;
    slot->captured=false;slot->client.disconnect();slot->route.reset();
    std::lock_guard<std::mutex> registry(browser_client_registry);if(browser_client==slot)browser_client.reset();return 0;
} catch(...){return 5;}

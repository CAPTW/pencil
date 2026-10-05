// Optional Writer client. The target's exact image/path must be supplied by a
// trusted build, never by a UNO caller, document or native protocol message.
// Reuses the authenticated one-review channel; default builds remain disabled.
#include "browser_bridge.hpp"
#include <mutex>
#ifndef GRAMMAR_TRUSTED_WRITER_SHA256
#define GRAMMAR_TRUSTED_WRITER_SHA256 ""
#endif
#ifndef GRAMMAR_TRUSTED_WRITER_PATH
#define GRAMMAR_TRUSTED_WRITER_PATH L""
#endif
namespace {
using namespace grammar;
struct WriterSlot {
    const uint64_t cookie;
    const HWND window;
    DWORD pid=0;
    std::mutex operation;
    std::atomic<bool> cancelled{false};
    BrowserBridgeClient client;
    bool opened=false,captured=false;
    WriterSlot(uint64_t value,HWND target):cookie(value),window(target){}
    bool target_alive(bool foreground=false) const {
        DWORD current=0;
        WCHAR name[32]{};
        return pid && IsWindow(window) && GetAncestor(window,GA_ROOT)==window &&
            GetWindowThreadProcessId(window,&current) && current==pid &&
            GetClassNameW(window,name,32) && !wcscmp(name,L"SALFRAME") &&
            IsWindowVisible(window) && !IsIconic(window) &&
            (!foreground || GetForegroundWindow()==window);
    }
};
std::mutex registry;
std::shared_ptr<WriterSlot> active;
uint64_t next_cookie=1;
std::shared_ptr<WriterSlot> lookup(uint64_t cookie){
    std::lock_guard<std::mutex> lock(registry);
    return active && active->cookie==cookie?active:nullptr;
}
bool trusted_digest(ApplyImageDigest& digest){
    const std::string value=GRAMMAR_TRUSTED_WRITER_SHA256;
    if(value.size()!=64 || !*GRAMMAR_TRUSTED_WRITER_PATH)return false;
    auto hex=[](char c){return c>='0'&&c<='9'?c-'0':c>='a'&&c<='f'?c-'a'+10:-1;};
    for(size_t i=0;i<digest.size();++i){
        const int a=hex(value[2*i]),b=hex(value[2*i+1]);
        if(a<0||b<0)return false;
        digest[i]=static_cast<BYTE>((a<<4)|b);
    }
    return true;
}
}
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarWriterClientCreate(HWND window,uint64_t* cookie) noexcept try {
    if(!window||!cookie)return 3;*cookie=0;
    ApplyImageDigest digest{};if(!trusted_digest(digest))return 4;
    std::lock_guard<std::mutex> lock(registry);
    if(active)return 2;if(next_cookie==UINT64_MAX)return 5;
    active=std::make_shared<WriterSlot>(next_cookie++,window);*cookie=active->cookie;return 0;
} catch(...){return 5;}
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarWriterClientCapture(uint64_t cookie,WCHAR* text,uint32_t capacity,uint32_t* units) noexcept try {
    if(!text||capacity<browser_bridge_units+1||!units)return 3;*units=0;text[0]=0;
    auto slot=lookup(cookie);if(!slot)return 1;
    std::unique_lock<std::mutex> lock(slot->operation,std::try_to_lock);if(!lock.owns_lock())return 2;
    if(slot->opened||slot->cancelled)return 4;slot->opened=true;
    const auto deadline=GetTickCount64()+3000;
    if(!GetWindowThreadProcessId(slot->window,&slot->pid)||!slot->target_alive(true))return 4;
    ApplyImageDigest digest{};if(!trusted_digest(digest))return 4;
    auto peer=verify_apply_peer(slot->pid,GRAMMAR_TRUSTED_WRITER_PATH,digest,deadline);
    if(!peer||slot->cancelled||!slot->target_alive(true)||!slot->client.connect(std::move(peer),deadline))return 4;
    TargetReply reply;std::wstring source;
    const bool ok=slot->client.request(ChannelCommand::capture,L"",reply,source,deadline)&&reply.status==2&&
        !slot->cancelled&&slot->target_alive(true)&&GetTickCount64()<deadline;
    if(ok){slot->captured=true;*units=static_cast<uint32_t>(source.size());memcpy(text,source.data(),source.size()*sizeof(wchar_t));text[*units]=0;}
    if(!source.empty())SecureZeroMemory(source.data(),source.size()*sizeof(wchar_t));
    if(!ok)slot->client.disconnect();return ok?0:4;
} catch(...){return 5;}
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarWriterClientApply(uint64_t cookie,const WCHAR* text,uint32_t units,uint32_t* outcome) noexcept try {
    if(!text||!units||units>browser_bridge_units||!outcome)return 3;*outcome=4;
    auto slot=lookup(cookie);if(!slot)return 1;
    std::unique_lock<std::mutex> lock(slot->operation,std::try_to_lock);if(!lock.owns_lock())return 2;
    if(!slot->captured||slot->cancelled||!slot->target_alive()){*outcome=0;return 0;}
    slot->captured=false;
    TargetReply reply;std::wstring ignored,replacement(text,units);
    const bool ok=slot->client.request(ChannelCommand::apply,replacement,reply,ignored);
    SecureZeroMemory(replacement.data(),replacement.size()*sizeof(wchar_t));
    if(ok)*outcome=reply.status;return ok?0:4;
} catch(...){return 5;}
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarWriterClientCancel(uint64_t cookie) noexcept try {
    auto slot=lookup(cookie);if(!slot)return 1;
    slot->cancelled=true;slot->client.cancel();
    std::unique_lock<std::mutex> lock(slot->operation,std::try_to_lock);
    if(lock.owns_lock()){slot->captured=false;slot->client.disconnect();}return 0;
} catch(...){return 5;}
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarWriterClientClose(uint64_t cookie,uint32_t milliseconds) noexcept try {
    if(milliseconds>8000)return 3;
    auto slot=lookup(cookie);if(!slot)return 1;
    slot->cancelled=true;slot->client.cancel();
    std::unique_lock<std::mutex> lock(slot->operation,std::try_to_lock);if(!lock.owns_lock())return 2;
    slot->captured=false;slot->client.disconnect();
    std::lock_guard<std::mutex> guard(registry);if(active==slot)active.reset();return 0;
} catch(...){return 5;}

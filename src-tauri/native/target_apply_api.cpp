// Narrow C ABI for the Rust backend. No frontend-supplied pointers, capture IDs
// or target handles may cross into this layer without backend token validation.
#include "target_apply_client.hpp"
#include <mutex>
using namespace grammar;
namespace {
struct Slot {
    const uint64_t cookie;
    const HWND field;
    TargetApplyClient client;
    std::mutex operation;
    uint64_t capture = 0;
    bool opened = false;
    Slot(uint64_t id, HWND window) : cookie(id),field(window) {}
};
std::mutex registry_mutex;
std::shared_ptr<Slot> current;
uint64_t next_cookie = 1;
std::shared_ptr<Slot> lookup(uint64_t id) {
    std::lock_guard<std::mutex> lock(registry_mutex);
    return current && current->cookie==id ? current : nullptr;
}
HMODULE this_module() {
    HMODULE module=nullptr;
    GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS|GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
        reinterpret_cast<LPCWSTR>(&this_module),&module);
    return module; // Rust keeps its LoadLibrary owner until all sessions close.
}
}
// Return codes:0 operation completed,1 unknown token,2 busy/cleanup pending,
//3 invalid input,4 transport/operation unavailable,5 internal failure.
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarTargetCreate(HWND field,uint64_t* cookie) noexcept try {
    if (!field || !cookie) return 3;
    *cookie=0;
    std::lock_guard<std::mutex> lock(registry_mutex);
    if (current) return 2;
    if (next_cookie==(std::numeric_limits<uint64_t>::max)()) return 5;
    current=std::make_shared<Slot>(next_cookie++,field); *cookie=current->cookie; return 0;
} catch (...) { return 5; }
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarTargetCapture(uint64_t cookie,WCHAR* text,uint32_t capacity,uint32_t* units) noexcept try {
    if (!text || capacity<65535 || !units) return 3;
    *units=0;
    auto slot=lookup(cookie); if (!slot) return 1;
    std::unique_lock<std::mutex> lock(slot->operation,std::try_to_lock); if (!lock.owns_lock()) return 2;
    if (slot->opened) return 3;
    slot->opened=true; slot->client.connect_to(slot->field,this_module());
    if (!slot->client.connected) return 4;
    TargetReply reply; std::wstring selected;
    if (!slot->client.request(ChannelCommand::capture,0,L"",reply,selected) || reply.status!=2 || selected.empty()) return 4;
    slot->capture=reply.capture_id; *units=static_cast<uint32_t>(selected.size());
    memcpy(text,selected.data(),selected.size()*sizeof(wchar_t));
    SecureZeroMemory(selected.data(),selected.size()*sizeof(wchar_t)); return 0;
} catch (...) { return 5; }
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarTargetApply(uint64_t cookie,const WCHAR* text,uint32_t units,uint32_t* outcome) noexcept try {
    if (!text || !units || units>65534 || !outcome) return 3;
    *outcome=4; // Unknown once transport may have delivered a mutation request.
    auto slot=lookup(cookie); if (!slot) return 1;
    std::unique_lock<std::mutex> lock(slot->operation,std::try_to_lock); if (!lock.owns_lock()) return 2;
    if (!slot->capture) { *outcome=0; return 0; }
    auto id=slot->capture; slot->capture=0; // Explicit user command, at most once.
    TargetReply reply; std::wstring ignored;
    if (!slot->client.request(ChannelCommand::apply,id,std::wstring(text,units),reply,ignored)) return 4;
    if (reply.status!=0 && reply.status!=3 && reply.status!=4) return 4;
    *outcome=reply.status; return 0;
} catch (...) { return 5; }
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarTargetCancel(uint64_t cookie) noexcept try {
    auto slot=lookup(cookie); if (!slot) return 1;
    slot->client.cancel();
    // An idle capture has no reader to observe the flag. Close its pipe now
    // without waiting, so the target lease revokes promptly on Dismiss/Copy.
    // A busy operation observes the atomic flag in its bounded I/O loop.
    std::unique_lock<std::mutex> lock(slot->operation,std::try_to_lock);
    if (lock.owns_lock()) (void)slot->client.close(0);
    return 0;
} catch (...) { return 5; }
extern "C" __declspec(dllexport) uint32_t WINAPI GrammarTargetClose(uint64_t cookie,uint32_t milliseconds) noexcept try {
    if (milliseconds>8000) return 3;
    auto slot=lookup(cookie); if (!slot) return 1;
    slot->client.cancel();
    std::unique_lock<std::mutex> lock(slot->operation,std::try_to_lock); if (!lock.owns_lock()) return 2;
    if (!slot->client.close(milliseconds)) return 2;
    std::lock_guard<std::mutex> registry(registry_mutex);
    if (current==slot) current.reset();
    return 0;
} catch (...) { return 5; }

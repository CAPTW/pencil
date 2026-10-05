#pragma once
#include "apply_channel.hpp"
namespace grammar {
inline constexpr WCHAR target_bootstrap_message[] = L"Grammar.TargetApply.Bootstrap.v1.7dc8373f";
inline std::wstring target_pipe_name(DWORD broker, DWORD target, uint64_t nonce) {
    return L"\\\\.\\pipe\\Grammar.Apply." + std::to_wstring(broker) + L"." + std::to_wstring(target) + L"." + std::to_wstring(nonce);
}
struct TargetHello {
    uint64_t magic = 0x31415941524d4152ULL;
    uint64_t window = 0;
    uint32_t version = 1, server_pid = 0, window_thread = 0, worker_thread = 0;
    ChannelCapability capability{};
};
static_assert(sizeof(TargetHello) == 64, "Exact Windows wire greeting");
struct TargetReply {
    uint32_t status = 0; // DispatchStatus values; never translate uncertainty to rejection.
    uint32_t units = 0;
    uint64_t capture_id = 0;
};
static_assert(sizeof(TargetReply) == 16, "Exact Windows wire response");
inline bool target_write(HANDLE pipe, const void* data, DWORD size, ULONGLONG deadline, const std::atomic<bool>* cancelled = nullptr) {
    auto cursor = static_cast<const BYTE*>(data);
    if ((!data && size) || size > channel_max_payload) return false;
    while (size && GetTickCount64() < deadline) {
        if (cancelled && cancelled->load()) return false;
        DWORD written = 0;
        if (!WriteFile(pipe, cursor, size > 1024 ? 1024 : size, &written, nullptr)) return false;
        cursor += written; size -= written;
        if (!written) Sleep(1);
    }
    return size == 0;
}
}

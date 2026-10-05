#pragma once
#include "target_apply_protocol.hpp"
namespace grammar {
// Broker-side synchronous operations belong on its blocking worker, never the
// widget event loop. Keep this owner until close() proves worker termination.
class TargetApplyClient final {
    ChannelHandle process_, worker_;
    HANDLE pipe_ = INVALID_HANDLE_VALUE;
    HHOOK hook_ = nullptr;
    ChannelCapability capability_{};
    uint64_t sequence_ = 1;
    std::atomic<bool> cancelled_{false};
    bool started_ = false, bootstrap_sent_ = false;
    static HANDLE own_process(DWORD id) { return OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, FALSE, id); }
public:
    bool connected = false;
    TargetApplyClient() = default;
    explicit TargetApplyClient(HWND field, HMODULE module) { connect_to(field,module); }
    void cancel() noexcept { cancelled_ = true; } // Safe during a blocking operation.
    void connect_to(HWND field, HMODULE module) {
        if (started_ || cancelled_) return;
        started_ = true;
        try { connect(field, module); }
        catch (...) { disconnect(); if (hook_) { UnhookWindowsHookEx(hook_); hook_ = nullptr; } throw; }
    }
private:
    void connect(HWND field, HMODULE module) {
        DWORD pid = 0, thread = GetWindowThreadProcessId(field, &pid);
        if (!pid || !thread || pid == GetCurrentProcessId()) return;
        process_.reset(own_process(pid)); HANDLE target = process_.get();
        if (!target) return;
        if (!same_local_identity(target)) return;
        uint64_t nonce = 0;
        if (BCryptGenRandom(nullptr, reinterpret_cast<BYTE*>(&nonce), sizeof(nonce), BCRYPT_USE_SYSTEM_PREFERRED_RNG) != 0 || !nonce) return;
        auto callback = reinterpret_cast<HOOKPROC>(GetProcAddress(module, "GrammarApplyHook"));
        if (!callback) return;
        if (cancelled_) return;
        hook_ = SetWindowsHookExW(WH_CALLWNDPROC, callback, module, thread);
        if (!hook_) return;
        DWORD_PTR result = 0;
        if (cancelled_) return;
        bootstrap_sent_ = true;
        if (!SendMessageTimeoutW(field, RegisterWindowMessageW(target_bootstrap_message), GetCurrentProcessId(), static_cast<LPARAM>(nonce),
            SMTO_ABORTIFHUNG | SMTO_BLOCK, 2000, &result)) return;
        auto name = target_pipe_name(GetCurrentProcessId(), pid, nonce);
        auto deadline = GetTickCount64() + 6000;
        do {
            pipe_ = CreateFileW(name.c_str(), GENERIC_READ | GENERIC_WRITE, 0, nullptr, OPEN_EXISTING,
                SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION, nullptr);
            if (pipe_ != INVALID_HANDLE_VALUE) break;
            if (GetLastError() != ERROR_FILE_NOT_FOUND && GetLastError() != ERROR_PIPE_BUSY) return;
            Sleep(5);
        } while (!cancelled_ && GetTickCount64() < deadline && WaitForSingleObject(target, 0) == WAIT_TIMEOUT);
        ULONG server = 0; DWORD mode = PIPE_NOWAIT;
        if (pipe_ == INVALID_HANDLE_VALUE || !GetNamedPipeServerProcessId(pipe_, &server) || server != pid ||
            !SetNamedPipeHandleState(pipe_, &mode, nullptr, nullptr)) return;
        TargetHello hello;
        if (!channel_read_exact(pipe_, &hello, sizeof(hello), deadline, &cancelled_)) { SecureZeroMemory(hello.capability.data(),hello.capability.size()); return; }
        if (hello.magic != TargetHello{}.magic || hello.version != 1 || hello.server_pid != pid || hello.window_thread != thread ||
            hello.window != reinterpret_cast<uintptr_t>(field)) { SecureZeroMemory(hello.capability.data(),hello.capability.size()); return; }
        worker_.reset(OpenThread(SYNCHRONIZE | THREAD_QUERY_LIMITED_INFORMATION, FALSE, hello.worker_thread));
        if (!worker_.get() || GetProcessIdOfThread(worker_.get()) != pid) { SecureZeroMemory(hello.capability.data(),hello.capability.size()); return; }
        capability_ = hello.capability; SecureZeroMemory(hello.capability.data(),hello.capability.size()); connected = true;
    }
public:
    TargetApplyClient(const TargetApplyClient&) = delete;
    TargetApplyClient& operator=(const TargetApplyClient&) = delete;
    ~TargetApplyClient() {
        disconnect();
        if (hook_) UnhookWindowsHookEx(hook_);
    }
    void disconnect() {
        connected = false;
        if (pipe_ != INVALID_HANDLE_VALUE) { CloseHandle(pipe_); pipe_ = INVALID_HANDLE_VALUE; }
        SecureZeroMemory(capability_.data(),capability_.size());
    }
    // false means cleanup remains pending/unobserved; retain the owner and
    // expose that state instead of calling timeout a completed cleanup.
    bool close(DWORD milliseconds) {
        cancel();
        disconnect();
        bool ended = worker_.get() ? WaitForSingleObject(worker_.get(), milliseconds) == WAIT_OBJECT_0 :
            !bootstrap_sent_ || (process_.get() && WaitForSingleObject(process_.get(), 0) == WAIT_OBJECT_0);
        if (ended && hook_) { if (!UnhookWindowsHookEx(hook_)) return false; hook_ = nullptr; }
        return ended;
    }
    bool request(ChannelCommand kind, uint64_t capture, const std::wstring& replacement, TargetReply& reply, std::wstring& text) {
        reply = {}; text.clear();
        if (!connected || cancelled_ || replacement.size() > 65534 || sequence_ == (std::numeric_limits<uint64_t>::max)()) return false;
        ChannelHeader header; header.command = kind; header.capture_id = capture; header.sequence = sequence_++;
        header.capability = capability_; header.payload_bytes = static_cast<DWORD>(replacement.size()*sizeof(wchar_t));
        auto deadline = GetTickCount64()+6500;
        bool written = target_write(pipe_, &header, sizeof(header), deadline, &cancelled_) && target_write(pipe_, replacement.data(), header.payload_bytes, deadline, &cancelled_);
        SecureZeroMemory(header.capability.data(),header.capability.size());
        if (!written || !channel_read_exact(pipe_, &reply, sizeof(reply), deadline, &cancelled_) || reply.status > 4 || reply.status == 1 ||
            reply.units > 65534 || (reply.status != 2 && reply.units) || (reply.status == 2 && (!reply.units || !reply.capture_id))) { disconnect(); return false; }
        text.resize(reply.units);
        if (!channel_read_exact(pipe_, text.data(), reply.units*sizeof(wchar_t), deadline, &cancelled_)) { text.clear(); disconnect(); return false; }
        return true;
    }
};
}

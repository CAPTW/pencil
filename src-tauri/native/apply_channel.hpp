#pragma once
// Experimental local transport admission. Activation must supply a trusted,
// live peer process handle and deliver the capability privately. This does not
// discover/trust arbitrary processes or grant permission to read a document.
#include <windows.h>
#include <bcrypt.h>
#include <array>
#include <cstdint>
#include <limits>
#include <vector>
#include <string>
#include <sddl.h>
#include <atomic>
#include <memory>

namespace grammar {
class ChannelHandle {
    HANDLE value_ = nullptr;
public:
    explicit ChannelHandle(HANDLE value = nullptr) : value_(value) {}
    ~ChannelHandle() { if (value_ && value_ != INVALID_HANDLE_VALUE) CloseHandle(value_); }
    ChannelHandle(const ChannelHandle&) = delete;
    ChannelHandle& operator=(const ChannelHandle&) = delete;
    HANDLE get() const { return value_; }
    HANDLE release() noexcept { HANDLE value = value_; value_ = nullptr; return value; }
    void reset(HANDLE value = nullptr) noexcept {
        if (value_ == value) return;
        if (value_ && value_ != INVALID_HANDLE_VALUE) CloseHandle(value_);
        value_ = value;
    }
};
enum class ChannelCommand : uint32_t { capture = 1, apply = 2, cancel = 3, disable = 4 };
using ChannelCapability = std::array<unsigned char, 32>;
struct ChannelHeader {
    uint32_t version = 2;
    ChannelCommand command = ChannelCommand::capture;
    uint64_t sequence = 1;
    uint64_t capture_id = 0; // Target-issued ID, never a caller-supplied HWND/range.
    uint32_t payload_bytes = 0;
    uint32_t reserved = 0;
    ChannelCapability capability{};
};
static_assert(sizeof(ChannelHeader) == 64, "Windows little-endian wire header must be exact");
static_assert(sizeof(wchar_t) == 2, "Wire text is UTF-16LE");
inline constexpr DWORD channel_max_payload = 65534 * sizeof(wchar_t);

class OwnedApplyPipe final {
    HANDLE pipe_ = INVALID_HANDLE_VALUE;
public:
    std::wstring name;
    explicit OwnedApplyPipe(const std::wstring& requestedName = {}) {
        HANDLE raw = nullptr;
        if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw)) return;
        ChannelHandle token(raw); DWORD size = 0;
        GetTokenInformation(token.get(), TokenGroups, nullptr, 0, &size);
        if (!size || size > 65536) return;
        std::vector<unsigned char> groups(size);
        if (!GetTokenInformation(token.get(), TokenGroups, groups.data(), size, &size)) return;
        auto info = reinterpret_cast<TOKEN_GROUPS*>(groups.data());
        std::wstring logon;
        for (DWORD i = 0; i < info->GroupCount; ++i) {
            if ((info->Groups[i].Attributes & SE_GROUP_LOGON_ID) == SE_GROUP_LOGON_ID) {
                LPWSTR sid = nullptr;
                if (!ConvertSidToStringSidW(info->Groups[i].Sid, &sid)) return;
                logon = sid; LocalFree(sid); break;
            }
        }
        if (logon.empty()) return;
        std::array<unsigned char, 16> random{};
        if (BCryptGenRandom(nullptr, random.data(), static_cast<ULONG>(random.size()), BCRYPT_USE_SYSTEM_PREFERRED_RNG) != 0) return;
        name = L"\\\\.\\pipe\\Grammar.Apply.";
        constexpr WCHAR digits[] = L"0123456789abcdef";
        for (auto byte : random) { name += digits[byte >> 4]; name += digits[byte & 15]; }
        if (!requestedName.empty()) {
            constexpr WCHAR prefix[] = L"\\\\.\\pipe\\Grammar.Apply.";
            if (requestedName.size() > 160 || requestedName.compare(0, ARRAYSIZE(prefix)-1, prefix) != 0) return;
            for (size_t i = ARRAYSIZE(prefix)-1; i < requestedName.size(); ++i)
                if (!(requestedName[i] == L'.' || (requestedName[i] >= L'0' && requestedName[i] <= L'9'))) return;
            name = requestedName;
        }
        auto sddl = L"D:P(A;;GA;;;" + logon + L")";
        PSECURITY_DESCRIPTOR descriptor = nullptr;
        if (!ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl.c_str(), SDDL_REVISION_1, &descriptor, nullptr)) return;
        SECURITY_ATTRIBUTES attributes{sizeof(attributes), descriptor, FALSE};
        pipe_ = CreateNamedPipeW(name.c_str(), PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1, 4096, 4096, 0, &attributes);
        LocalFree(descriptor);
    }
    ~OwnedApplyPipe() { if (pipe_ != INVALID_HANDLE_VALUE) CloseHandle(pipe_); }
    OwnedApplyPipe(const OwnedApplyPipe&) = delete;
    OwnedApplyPipe& operator=(const OwnedApplyPipe&) = delete;
    HANDLE get() const { return pipe_; }
};

inline bool same_local_identity(HANDLE process) {
    HANDLE rawSelf = nullptr, rawPeer = nullptr;
    if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &rawSelf)) return false;
    ChannelHandle self(rawSelf);
    if (!OpenProcessToken(process, TOKEN_QUERY, &rawPeer)) return false;
    ChannelHandle peer(rawPeer);
    auto query = [](HANDLE token, TOKEN_INFORMATION_CLASS kind, std::vector<unsigned char>& data) {
        DWORD size = 0;
        GetTokenInformation(token, kind, nullptr, 0, &size);
        if (!size || size > 65536) return false;
        data.resize(size);
        return GetTokenInformation(token, kind, data.data(), size, &size) != FALSE;
    };
    std::vector<unsigned char> a, b;
    if (!query(self.get(), TokenUser, a) || !query(peer.get(), TokenUser, b) ||
        !EqualSid(reinterpret_cast<TOKEN_USER*>(a.data())->User.Sid,
                  reinterpret_cast<TOKEN_USER*>(b.data())->User.Sid)) return false;
    TOKEN_STATISTICS statsA{}, statsB{}; DWORD returned = 0, sessionA = 0, sessionB = 0;
    if (!GetTokenInformation(self.get(), TokenStatistics, &statsA, sizeof(statsA), &returned) ||
        !GetTokenInformation(peer.get(), TokenStatistics, &statsB, sizeof(statsB), &returned) ||
        statsA.AuthenticationId.HighPart != statsB.AuthenticationId.HighPart ||
        statsA.AuthenticationId.LowPart != statsB.AuthenticationId.LowPart ||
        !GetTokenInformation(self.get(), TokenSessionId, &sessionA, sizeof(sessionA), &returned) ||
        !GetTokenInformation(peer.get(), TokenSessionId, &sessionB, sizeof(sessionB), &returned) ||
        sessionA != sessionB) return false;
    if (!query(self.get(), TokenIntegrityLevel, a) || !query(peer.get(), TokenIntegrityLevel, b)) return false;
    auto level = [](const std::vector<unsigned char>& data) {
        auto sid = reinterpret_cast<const TOKEN_MANDATORY_LABEL*>(data.data())->Label.Sid;
        return *GetSidSubAuthority(sid, *GetSidSubAuthorityCount(sid) - 1);
    };
    // Initial implementation supports neither elevated nor low-integrity peers.
    return level(a) == SECURITY_MANDATORY_MEDIUM_RID && level(b) == SECURITY_MANDATORY_MEDIUM_RID;
}

// Safe for the editor STA to consult while the I/O worker owns admission. Both
// kernel handles are retained, preventing reuse, and revocation is atomic.
class ApplyConnectionLease final {
    ChannelHandle process_, pipe_;
    std::atomic<bool> revoked_{false};
    static HANDLE duplicate(HANDLE handle) {
        HANDLE value = nullptr;
        if (!DuplicateHandle(GetCurrentProcess(), handle, GetCurrentProcess(), &value, 0, FALSE, DUPLICATE_SAME_ACCESS)) return nullptr;
        return value;
    }
public:
    ApplyConnectionLease(HANDLE process, HANDLE pipe) : process_(duplicate(process)), pipe_(duplicate(pipe)) {}
    void revoke() { revoked_.store(true); }
    bool alive() const {
        ULONG pid = 0;
        return !revoked_.load() && process_.get() && pipe_.get() &&
            WaitForSingleObject(process_.get(), 0) == WAIT_TIMEOUT &&
            PeekNamedPipe(pipe_.get(), nullptr, 0, nullptr, nullptr, nullptr) &&
            GetNamedPipeClientProcessId(pipe_.get(), &pid) && pid == GetProcessId(process_.get());
    }
};

class ApplyChannelAdmission final {
    ChannelHandle peer_;
    ChannelCapability capability_{};
    uint64_t next_ = 1;
    bool enabled_ = false;
    std::shared_ptr<ApplyConnectionLease> lease_;
    static HANDLE duplicate(HANDLE process) {
        HANDLE value = nullptr;
        if (!DuplicateHandle(GetCurrentProcess(), process, GetCurrentProcess(), &value,
            SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, FALSE, 0)) return nullptr;
        return value;
    }
public:
    // Target I/O worker owns this object; never concurrently access/revoke it.
    explicit ApplyChannelAdmission(HANDLE trustedPeer) : peer_(duplicate(trustedPeer)) {
        enabled_ = peer_.get() && WaitForSingleObject(peer_.get(), 0) == WAIT_TIMEOUT &&
            same_local_identity(peer_.get()) &&
            BCryptGenRandom(nullptr, capability_.data(), static_cast<ULONG>(capability_.size()), BCRYPT_USE_SYSTEM_PREFERRED_RNG) == 0;
    }
    ~ApplyChannelAdmission() { revoke(); }
    const ChannelCapability& bootstrap_capability() const { return capability_; }
    bool alive() const { return enabled_ && WaitForSingleObject(peer_.get(), 0) == WAIT_TIMEOUT; }
    void revoke() { enabled_ = false; if (lease_) lease_->revoke(); SecureZeroMemory(capability_.data(), capability_.size()); }
    std::shared_ptr<ApplyConnectionLease> lease() const { return lease_; } // I/O owner only.
    bool peer_matches(HANDLE serverPipe) const {
        ULONG actual = 0;
        return alive() && PeekNamedPipe(serverPipe, nullptr, 0, nullptr, nullptr, nullptr) &&
            GetNamedPipeClientProcessId(serverPipe, &actual) && actual == GetProcessId(peer_.get());
    }
    bool admit(HANDLE serverPipe, const ChannelHeader& header) {
        unsigned difference = 0;
        for (size_t i = 0; i < capability_.size(); ++i) difference |= capability_[i] ^ header.capability[i];
        const bool known = header.command == ChannelCommand::capture || header.command == ChannelCommand::apply ||
            header.command == ChannelCommand::cancel || header.command == ChannelCommand::disable;
        const bool body = header.command == ChannelCommand::apply ?
            header.payload_bytes > 0 && header.payload_bytes <= channel_max_payload && header.payload_bytes % sizeof(wchar_t) == 0 :
            header.payload_bytes == 0;
        const bool reference = header.command == ChannelCommand::apply || header.command == ChannelCommand::cancel ?
            header.capture_id != 0 : header.capture_id == 0;
        if (!peer_matches(serverPipe) || difference || header.version != 2 || header.reserved || !reference ||
            !known || !body || header.sequence != next_ || next_ == (std::numeric_limits<uint64_t>::max)()) {
            revoke(); return false;
        }
        ++next_;
        if (!lease_) lease_ = std::make_shared<ApplyConnectionLease>(peer_.get(), serverPipe);
        if (!lease_->alive()) { revoke(); return false; }
        if (header.command == ChannelCommand::disable) revoke();
        return true;
    }
};

// Nonblocking byte-mode pipe operations. One absolute deadline is passed through
// header and payload, never restarted per chunk. No pending OVERLAPPED storage,
// detached reader or unbounded cleanup wait; only call on the transport worker.
inline bool channel_read_exact(HANDLE pipe, void* output, DWORD bytes, ULONGLONG deadline, const std::atomic<bool>* cancelled = nullptr) {
    DWORD mode = 0;
    if ((bytes && !output) || bytes > channel_max_payload || !GetNamedPipeHandleStateW(pipe, &mode, nullptr, nullptr, nullptr, nullptr, 0) ||
        !(mode & PIPE_NOWAIT)) return false;
    auto cursor = static_cast<unsigned char*>(output);
    while (bytes) {
        if (GetTickCount64() >= deadline || (cancelled && cancelled->load())) return false;
        DWORD received = 0;
        if (ReadFile(pipe, cursor, bytes, &received, nullptr)) {
            if (!received) return false;
            cursor += received; bytes -= received;
        } else {
            if (GetLastError() != ERROR_NO_DATA) return false;
            Sleep(1);
        }
    }
    return true;
}
struct ChannelRequest {
    ChannelRequest() = default;
    ChannelRequest(const ChannelRequest&) = delete;
    ChannelRequest& operator=(const ChannelRequest&) = delete;
    ChannelHeader header;
    std::vector<unsigned char> body;
    void clear() {
        if (!body.empty()) SecureZeroMemory(body.data(), body.size());
        body.clear(); SecureZeroMemory(header.capability.data(), header.capability.size()); header = ChannelHeader{};
    }
    ~ChannelRequest() { clear(); }
};
inline bool channel_valid_text(const std::vector<unsigned char>& body) {
    if (body.empty() || body.size() % 2) return false;
    auto unit = [&body](size_t i) { return static_cast<unsigned>(body[i]) | (static_cast<unsigned>(body[i + 1]) << 8); };
    for (size_t i = 0; i < body.size(); i += 2) {
        unsigned c = unit(i);
        if (!c || (c >= 0xdc00 && c <= 0xdfff)) return false;
        if (c >= 0xd800 && c <= 0xdbff) {
            if (i + 3 >= body.size()) return false;
            unsigned low = unit(i + 2);
            if (low < 0xdc00 || low > 0xdfff) return false;
            i += 2;
        }
    }
    return true;
}
inline bool receive_apply_request(HANDLE pipe, ApplyChannelAdmission& admission,
                                  ChannelRequest& request, ULONGLONG deadline) try {
    request.clear();
    // No payload allocation or reading before peer identity and header admission.
    if (!admission.peer_matches(pipe) || !channel_read_exact(pipe, &request.header, sizeof(request.header), deadline) ||
        !admission.admit(pipe, request.header)) { admission.revoke(); request.clear(); return false; }
    SecureZeroMemory(request.header.capability.data(), request.header.capability.size());
    request.body.resize(request.header.payload_bytes);
    if (!channel_read_exact(pipe, request.body.data(), request.header.payload_bytes, deadline)) {
        admission.revoke(); request.clear(); return false;
    }
    if (GetTickCount64() >= deadline ||
        (request.header.command == ChannelCommand::apply && !channel_valid_text(request.body)) ||
        (request.header.command != ChannelCommand::disable && !admission.peer_matches(pipe))) {
        admission.revoke(); request.clear(); return false;
    }
    return true;
} catch (...) { admission.revoke(); request.clear(); return false; }
}

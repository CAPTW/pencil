#pragma once
// Image identity supplements kernel process/token checks. The expected path
// and digest MUST come from the trusted build/install boundary, never an IPC
// message, caller-provided manifest, or the process being admitted.
#include "apply_channel.hpp"
#include <new>

namespace grammar {
using ApplyImageDigest = std::array<unsigned char, 32>;
inline bool apply_image_digest(HANDLE file, ApplyImageDigest& digest, ULONGLONG deadline) {
    digest.fill(0);
    LARGE_INTEGER size{}, start{};
    if (GetFileType(file) != FILE_TYPE_DISK || !GetFileSizeEx(file, &size) || size.QuadPart <= 0 ||
        size.QuadPart > 256LL * 1024 * 1024 || !SetFilePointerEx(file, start, nullptr, FILE_BEGIN)) return false;
    BCRYPT_ALG_HANDLE algorithm = nullptr; BCRYPT_HASH_HANDLE hash = nullptr;
    if (BCryptOpenAlgorithmProvider(&algorithm, BCRYPT_SHA256_ALGORITHM, nullptr, 0) != 0) return false;
    bool valid = BCryptCreateHash(algorithm, &hash, nullptr, 0, nullptr, 0, 0) == 0;
    std::array<unsigned char, 65536> buffer{};
    LONGLONG remaining = size.QuadPart;
    while (valid && remaining > 0) {
        if (GetTickCount64() >= deadline) { valid = false; break; }
        DWORD count = 0;
        DWORD wanted = static_cast<DWORD>(remaining > static_cast<LONGLONG>(buffer.size()) ? buffer.size() : remaining);
        valid = ReadFile(file, buffer.data(), wanted, &count, nullptr) && count == wanted &&
            BCryptHashData(hash, buffer.data(), count, 0) == 0;
        remaining -= count;
    }
    valid = valid && remaining == 0 && GetTickCount64() < deadline &&
        BCryptFinishHash(hash, digest.data(), static_cast<ULONG>(digest.size()), 0) == 0;
    if (hash) BCryptDestroyHash(hash);
    BCryptCloseAlgorithmProvider(algorithm, 0);
    if (!valid) digest.fill(0);
    return valid;
}
inline std::wstring apply_final_path(HANDLE file) {
    std::wstring path(32768, L'\0');
    DWORD size = GetFinalPathNameByHandleW(file, path.data(), static_cast<DWORD>(path.size()), FILE_NAME_NORMALIZED | VOLUME_NAME_DOS);
    if (!size || size >= path.size()) return {};
    path.resize(size); return path;
}
class VerifiedApplyPeer final {
    ChannelHandle process_, image_;
    friend std::unique_ptr<VerifiedApplyPeer> verify_apply_peer(DWORD, const std::wstring&, const ApplyImageDigest&, ULONGLONG);
    VerifiedApplyPeer(HANDLE process, HANDLE image) noexcept : process_(process), image_(image) {}
public:
    HANDLE process() const { return process_.get(); }
    bool alive() const { return WaitForSingleObject(process_.get(), 0) == WAIT_TIMEOUT; }
};
inline std::unique_ptr<VerifiedApplyPeer> verify_apply_peer(DWORD pid, const std::wstring& trustedImage,
                                                          const ApplyImageDigest& expected, ULONGLONG deadline) try {
    if (!pid || pid == GetCurrentProcessId() || trustedImage.empty() || GetTickCount64() >= deadline) return {};
    HANDLE process = OpenProcess(SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
    if (!process) return {};
    ChannelHandle processGuard(process);
    if (WaitForSingleObject(process, 0) != WAIT_TIMEOUT || !same_local_identity(process)) return {};
    std::wstring actual(32768, L'\0'); DWORD length = static_cast<DWORD>(actual.size());
    if (!QueryFullProcessImageNameW(process, 0, actual.data(), &length) || !length || length >= actual.size()) return {};
    actual.resize(length);
    // Retain an opened image without write/delete sharing throughout admission.
    ChannelHandle actualFile(CreateFileW(actual.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL | FILE_FLAG_SEQUENTIAL_SCAN, nullptr));
    ChannelHandle expectedFile(CreateFileW(trustedImage.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL, nullptr));
    if (actualFile.get() == INVALID_HANDLE_VALUE || expectedFile.get() == INVALID_HANDLE_VALUE) return {};
    auto actualPath = apply_final_path(actualFile.get()), expectedPath = apply_final_path(expectedFile.get());
    if (actualPath.empty() || expectedPath.empty() ||
        CompareStringOrdinal(actualPath.c_str(), -1, expectedPath.c_str(), -1, TRUE) != CSTR_EQUAL) return {};
    ApplyImageDigest observed{};
    if (!apply_image_digest(actualFile.get(), observed, deadline)) return {};
    unsigned difference = 0;
    for (size_t i = 0; i < observed.size(); ++i) difference |= observed[i] ^ expected[i];
    if (difference || WaitForSingleObject(process, 0) != WAIT_TIMEOUT || GetTickCount64() >= deadline) return {};
    // Constructor cannot throw. Guards transfer only after allocation succeeds.
    auto result = new(std::nothrow) VerifiedApplyPeer(process, actualFile.get());
    if (result) { processGuard.release(); actualFile.release(); }
    return std::unique_ptr<VerifiedApplyPeer>(result);
} catch (...) { return {}; }
}

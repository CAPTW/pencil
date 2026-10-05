#pragma once
// Browser/host process routing only. This never authorizes document capture or
// mutation: the MV3 worker must also bind documentId/epoch and the target-issued
// selection review. A service worker's --parent-window=0 is not a window ID.
#include "apply_peer.hpp"
#include <tlhelp32.h>
namespace grammar {
inline DWORD browser_host_parent(DWORD host, ULONGLONG deadline) {
    ChannelHandle snapshot(CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0));
    if (snapshot.get() == INVALID_HANDLE_VALUE) return 0;
    PROCESSENTRY32W entry{};entry.dwSize=sizeof(entry);
    if (!Process32FirstW(snapshot.get(), &entry)) return 0;
    unsigned visited=0;
    do {
        if (++visited>65536 || GetTickCount64()>=deadline) return 0;
        if (entry.th32ProcessID==host) return entry.th32ParentProcessID;
    } while (Process32NextW(snapshot.get(), &entry));
    return 0;
}
inline bool browser_created_before(HANDLE browser, HANDLE host) {
    FILETIME a{},b{},exit{},kernel{},user{};
    return GetProcessTimes(browser,&a,&exit,&kernel,&user) &&
        GetProcessTimes(host,&b,&exit,&kernel,&user) && CompareFileTime(&a,&b)<=0;
}
class BrowserProcessRoute final {
    std::unique_ptr<VerifiedApplyPeer> host_, launcher_;
    ChannelHandle browser_;
    HWND window_;
    BrowserProcessRoute(std::unique_ptr<VerifiedApplyPeer> host, std::unique_ptr<VerifiedApplyPeer> launcher, HANDLE browser, HWND window)
        :host_(std::move(host)),launcher_(std::move(launcher)),browser_(browser),window_(window) {}
    friend std::unique_ptr<BrowserProcessRoute> verify_browser_route(DWORD,HWND,const std::wstring&,const ApplyImageDigest&,ULONGLONG);
public:
    BrowserProcessRoute(const BrowserProcessRoute&)=delete;
    BrowserProcessRoute& operator=(const BrowserProcessRoute&)=delete;
    bool alive() const {
        DWORD pid=0;
        return host_ && host_->alive() && (!launcher_ || launcher_->alive()) && WaitForSingleObject(browser_.get(),0)==WAIT_TIMEOUT &&
            IsWindow(window_) && GetAncestor(window_,GA_ROOT)==window_ &&
            GetWindowThreadProcessId(window_,&pid) && pid==GetProcessId(browser_.get());
    }
    bool foreground() const {return alive() && GetForegroundWindow()==window_ && IsWindowVisible(window_) && !IsIconic(window_);}
    DWORD host_pid() const {return GetProcessId(host_->process());}
    DWORD browser_pid() const {return GetProcessId(browser_.get());}
};
inline std::unique_ptr<BrowserProcessRoute> verify_browser_route(DWORD host,HWND window,
    const std::wstring& trustedHostImage,const ApplyImageDigest& trustedHostDigest,ULONGLONG deadline) try {
    // Identity is supplied by the desktop's trusted installation and foreground
    // observation, never by an extension field or a caller-supplied file path.
    if (!window || window!=GetForegroundWindow() || GetAncestor(window,GA_ROOT)!=window ||
        !IsWindowVisible(window) || IsIconic(window) || GetTickCount64()>=deadline) return {};
    WCHAR klass[64]{};
    if (!GetClassNameW(window,klass,64) || wcscmp(klass,L"Chrome_WidgetWin_1")) return {};
    DWORD browser=0;
    if (!GetWindowThreadProcessId(window,&browser) || !browser || browser==host || browser==GetCurrentProcessId()) return {};
    auto peer=verify_apply_peer(host,trustedHostImage,trustedHostDigest,deadline);
    if (!peer) return {};
    std::unique_ptr<VerifiedApplyPeer> launcher;
    const DWORD parent=browser_host_parent(host,deadline);
    if (parent!=browser) {
        // Chromium may launch executable native hosts through exactly one
        // Windows cmd.exe shim. Do not accept an arbitrary ancestor chain or
        // merely a process named cmd.exe. Pin the canonical system image.
        WCHAR system[MAX_PATH]{};
        const auto length=GetSystemDirectoryW(system,MAX_PATH);
        if (!length || length>=MAX_PATH || !parent) return {};
        const auto cmd=std::wstring(system)+L"\\cmd.exe";
        ChannelHandle image(CreateFileW(cmd.c_str(),GENERIC_READ,FILE_SHARE_READ,nullptr,OPEN_EXISTING,FILE_ATTRIBUTE_NORMAL,nullptr));
        ApplyImageDigest hash{};
        if (image.get()==INVALID_HANDLE_VALUE || !apply_image_digest(image.get(),hash,deadline)) return {};
        launcher=verify_apply_peer(parent,cmd,hash,deadline);
        if (!launcher || browser_host_parent(parent,deadline)!=browser ||
            !browser_created_before(launcher->process(),peer->process())) return {};
    }
    ChannelHandle process(OpenProcess(SYNCHRONIZE|PROCESS_QUERY_LIMITED_INFORMATION,FALSE,browser));
    if (!process.get() || WaitForSingleObject(process.get(),0)!=WAIT_TIMEOUT || !same_local_identity(process.get()) ||
        !browser_created_before(process.get(),peer->process()) ||
        (launcher && !browser_created_before(process.get(),launcher->process())) || GetTickCount64()>=deadline) return {};
    auto result=std::unique_ptr<BrowserProcessRoute>(new BrowserProcessRoute(std::move(peer),std::move(launcher),process.release(),window));
    if (!result->foreground() || GetTickCount64()>=deadline) return {};
    return result;
} catch (...) {return {};}
} // namespace grammar

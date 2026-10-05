#pragma once
#include <windows.h>
#include <msctf.h>
#include <olectl.h>
#include <wrl/client.h>

namespace grammar {
// One subscription per owner. An unsuccessful close retains the exact source
// and cookie for caller-controlled retirement; it never means cleanup complete.
// The sink must separately retain the runtime callback-lifetime token.
class TsfSinkSubscription final {
    const DWORD thread_ = GetCurrentThreadId();
    Microsoft::WRL::ComPtr<ITfSource> source_;
    DWORD cookie_ = TF_INVALID_COOKIE;
    bool attempted_ = false, unresolved_ = false, in_call_ = false;
    HRESULT last_ = S_OK;
public:
    TsfSinkSubscription() = default;
    TsfSinkSubscription(const TsfSinkSubscription&) = delete;
    TsfSinkSubscription& operator=(const TsfSinkSubscription&) = delete;
    bool attach(ITfSource* source, REFIID type, IUnknown* sink) {
        if (GetCurrentThreadId() != thread_ || attempted_ || in_call_ || !source || !sink) return false;
        attempted_ = true; source_ = source; in_call_ = true;
        last_ = source_->AdviseSink(type, sink, &cookie_);
        in_call_ = false;
        // Failed AdviseSink normally returns INVALID_COOKIE. A provider that
        // supplies a cookie despite failure still requires explicit retirement.
        unresolved_ = cookie_ != TF_INVALID_COOKIE || SUCCEEDED(last_);
        if (!unresolved_) source_.Reset();
        return last_ == S_OK && cookie_ != TF_INVALID_COOKIE;
    }
    bool close() {
        if (GetCurrentThreadId() != thread_ || in_call_) return false;
        if (!unresolved_) return true;
        if (!source_ || cookie_ == TF_INVALID_COOKIE) return false;
        in_call_ = true;
        last_ = source_->UnadviseSink(cookie_);
        in_call_ = false;
        if (last_ != S_OK && last_ != CONNECT_E_NOCONNECTION) return false;
        cookie_ = TF_INVALID_COOKIE; unresolved_ = false; source_.Reset();
        return true;
    }
    bool disconnected() const { return !in_call_ && !unresolved_; }
    HRESULT last_result() const { return last_; }
    // Best effort only. The runtime must call close(), inspect its result and
    // retain its actor/module on failure rather than relying on this destructor.
    ~TsfSinkSubscription() { close(); }
};
}

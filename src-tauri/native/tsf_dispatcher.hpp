#pragma once
#include "tsf_capture.hpp"
#include "apply_channel.hpp"

namespace grammar {
enum class DispatchStatus : uint32_t { rejected, pending, captured, applied, mutation_unconfirmed };
// Target STA only. Transport admission is separate; no untrusted HWND/context,
// source or range enters through the wire. At most one TSF operation is pending.
class TsfTargetDispatcher final {
    const DWORD thread_ = GetCurrentThreadId();
    const TfClientId client_;
    std::shared_ptr<void> callback_lifetime_;
    std::unique_ptr<TsfDocumentBinding> binding_;
    ComPtr<TsfCaptureSession> capture_;
    ComPtr<TsfApplySession> apply_;
    std::shared_ptr<ApplyConnectionLease> lease_;
    uint64_t capture_id_ = 0;
    uint64_t next_capture_id_ = 1;
    bool consumed_ = true;
    bool on_thread() const { return GetCurrentThreadId() == thread_; }
    bool pending() const { return (capture_ && !capture_->completed) || (apply_ && !apply_->completed); }
    void clear_text() {
        if (capture_ && !capture_->source.empty()) {
            SecureZeroMemory(capture_->source.data(), capture_->source.size() * sizeof(wchar_t));
            capture_->source.clear();
        }
    }
public:
    explicit TsfTargetDispatcher(TfClientId client, std::shared_ptr<void> callbackLifetime = {})
        : client_(client), callback_lifetime_(std::move(callbackLifetime)) {}
    ~TsfTargetDispatcher() { cancel(); clear_text(); }
    TsfTargetDispatcher(const TsfTargetDispatcher&) = delete;
    TsfTargetDispatcher& operator=(const TsfTargetDispatcher&) = delete;
    bool executing() const { return (capture_ && capture_->called && !capture_->completed) || (apply_ && apply_->called && !apply_->completed); }
    void retire_completed_apply() {
        if (!on_thread() || !apply_ || !apply_->completed) return;
        cancel(); apply_.Reset(); capture_.Reset(); binding_.reset(); lease_.reset();
    }
    void cancel() {
        if (!on_thread()) return;
        consumed_ = true; capture_id_ = 0;
        if (binding_) binding_->disable();
        clear_text();
        // Keep pending COM owners until callback completion; no new operation
        // is admitted while a cancelled callback still owns its context.
    }
    bool cancel_capture(uint64_t id) {
        if (!on_thread() || !id || id != capture_id_ || (consumed_ && (!apply_ || apply_->completed))) return false;
        cancel(); return true;
    }
    bool begin_capture(ITfContext* trustedContext, HWND trustedWindow,
                       bool explicitOptIn, std::shared_ptr<ApplyConnectionLease> lease,
                       std::chrono::steady_clock::time_point deadline=std::chrono::steady_clock::now()+std::chrono::seconds(5)) {
        if (!on_thread() || next_capture_id_ == (std::numeric_limits<uint64_t>::max)() ||
            !lease || !lease->alive() || pending() || std::chrono::steady_clock::now()>=deadline) return false;
        cancel(); apply_.Reset(); capture_.Reset(); binding_.reset();
        if (!explicitOptIn) return false;
        lease_ = std::move(lease);
        binding_ = std::make_unique<TsfDocumentBinding>(trustedContext, trustedWindow, true);
        if (!binding_->state->enabled) return false;
        std::weak_ptr<ApplyConnectionLease> weak = lease_;
        capture_.Attach(new TsfCaptureSession(binding_->state, [weak] {
            auto link = weak.lock(); return link && link->alive();
        }, callback_lifetime_,deadline));
        capture_id_ = next_capture_id_++;
        HRESULT result = E_PENDING;
        HRESULT request = trustedContext->RequestEditSession(client_, capture_.Get(), TF_ES_ASYNC | TF_ES_READ, &result);
        // A synchronous callback can return a failure after executing. Preserve
        // its observable outcome instead of disguising it as admission failure.
        if (!capture_->completed && (FAILED(request) || result != TF_S_ASYNC)) { cancel(); capture_.Reset(); return false; }
        consumed_ = false;
        return true;
    }
    DispatchStatus capture_status() {
        if (!on_thread() || consumed_ || !capture_ || !lease_ || !lease_->alive()) { cancel(); return DispatchStatus::rejected; }
        if (!capture_->completed) return DispatchStatus::pending;
        if (!capture_->captured || !capture_->grant || !capture_->grant->valid.load()) { cancel(); return DispatchStatus::rejected; }
        return DispatchStatus::captured;
    }
    bool selected_text(uint64_t id, std::wstring& text) {
        text.clear();
        if (id != capture_id_ || capture_status() != DispatchStatus::captured) return false;
        text = capture_->source.substr(capture_->start, capture_->end - capture_->start);
        return true; // Full source remains inside the target process.
    }
    uint64_t capture_id() const { return consumed_ ? 0 : capture_id_; }
    bool can_apply(uint64_t id) {
        return on_thread() && !pending() && id != 0 && id == capture_id_ && capture_status() == DispatchStatus::captured;
    }
    bool begin_apply(uint64_t id, std::wstring replacement,
        std::chrono::steady_clock::time_point deadline=std::chrono::steady_clock::now()+std::chrono::seconds(5)) {
        if (!can_apply(id) || std::chrono::steady_clock::now()>=deadline) return false;
        auto grant = capture_->grant;
        grant->deadline=(std::min)(grant->deadline,deadline);
        auto verify = grant->verify_binding;
        std::weak_ptr<ApplyConnectionLease> weak = lease_;
        grant->verify_binding = [verify, weak](ITfContext* context, TfEditCookie cookie) {
            auto link = weak.lock();
            return link && link->alive() && verify && verify(context, cookie);
        };
        apply_.Attach(new TsfApplySession(binding_->state->context.Get(), capture_->source,
            capture_->start, capture_->end, std::move(replacement), grant->revision.load(), grant, callback_lifetime_));
        consumed_ = true; clear_text(); // Retain ID solely for queued cancellation.
        HRESULT result = E_PENDING;
        HRESULT request = binding_->state->context->RequestEditSession(client_, apply_.Get(), TF_ES_ASYNC | TF_ES_READWRITE, &result);
        if (!apply_->completed && (FAILED(request) || result != TF_S_ASYNC)) { cancel(); apply_.Reset(); return false; }
        return true;
    }
    DispatchStatus apply_status() const {
        if (!on_thread() || !apply_) return DispatchStatus::rejected;
        if (!apply_->completed) return DispatchStatus::pending;
        switch (apply_->result) {
        case ApplyResult::applied: return DispatchStatus::applied;
        case ApplyResult::mutation_unconfirmed: return DispatchStatus::mutation_unconfirmed;
        default: return DispatchStatus::rejected;
        }
    }
    unsigned mutation_calls() const { return apply_ ? apply_->mutation_calls : 0; }
    unsigned eligibility_stage() const { return binding_ ? binding_->state->eligibility_stage : 0; }
    unsigned capture_reads() const { return capture_ ? capture_->text_reads : 0; }
    bool capture_called() const { return capture_ && capture_->called; }
};
}

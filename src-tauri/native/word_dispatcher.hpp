#pragma once
#include "word_edit_session.hpp"
#include <limits>

namespace grammar {
enum class WordDispatchStatus { rejected, pending, captured, applied, mutation_unconfirmed };
// Owning-STA actor for a future authenticated Word route. Context/window/port
// and authority are supplied by the trusted target service, NEVER wire input.
// No production class admission is enabled by this component.
class WordTargetDispatcher final {
    const DWORD thread_ = GetCurrentThreadId();
    const TfClientId client_;
    std::shared_ptr<void> lifetime_;
    std::unique_ptr<WordTsfRevisionBinding> binding_;
    ComPtr<WordEditSession> capture_, apply_;
    uint64_t id_ = 0, next_ = 1;
    bool consumed_ = true;
    bool owner() const { return GetCurrentThreadId() == thread_; }
    bool request(WordEditSession* session, DWORD flags) {
        HRESULT disposition = E_PENDING;
        const auto hr = binding_->state->context->RequestEditSession(client_, session, TF_ES_ASYNC | flags, &disposition);
        if (session->completed || session->called) return true;
        if (FAILED(hr) || disposition != TF_S_ASYNC) { session->cancel(); return false; }
        return true;
    }
public:
    WordTargetDispatcher(TfClientId client, std::shared_ptr<void> callbackLifetime)
        : client_(client), lifetime_(std::move(callbackLifetime)) {}
    WordTargetDispatcher(const WordTargetDispatcher&) = delete;
    WordTargetDispatcher& operator=(const WordTargetDispatcher&) = delete;
    bool pending() const {
        return (capture_ && !capture_->completed) || (apply_ && !apply_->completed);
    }
    void cancel() {
        if (!owner()) return;
        consumed_ = true; id_ = 0;
        if (capture_) capture_->cancel();
        if (apply_) apply_->cancel();
        if (binding_) binding_->disable();
    }
    bool cancel_capture(uint64_t id) {
        if (!owner() || !id || id != id_) return false;
        cancel(); return true;
    }
    bool close() {
        if (!owner()) return false;
        cancel();
        if (pending()) return false; // Retain callback/state until actual completion.
        if (binding_ && !binding_->close()) return false; // Retain unresolved cookie.
        apply_.Reset(); capture_.Reset(); binding_.reset();
        return true;
    }
    bool begin_capture(ITfContext* context, HWND field, HWND root, std::shared_ptr<WordNativeWindowPort> port,
        bool explicitOptIn, std::function<bool()> authority, std::chrono::steady_clock::time_point deadline) {
        if (!owner() || !lifetime_ || !context || !port || !explicitOptIn || !authority || !authority() ||
            std::chrono::steady_clock::now() >= deadline || next_ == (std::numeric_limits<uint64_t>::max)() || pending()) return false;
        if (!close()) return false;
        binding_ = std::make_unique<WordTsfRevisionBinding>(context, field, root, std::move(port), true, lifetime_, std::move(authority));
        if (!binding_->state->enabled) return false;
        capture_.Attach(new WordEditSession(binding_->state, lifetime_, deadline));
        id_ = next_++; consumed_ = false;
        if (!request(capture_.Get(), TF_ES_READ)) { cancel(); capture_.Reset(); return false; }
        return true;
    }
    WordDispatchStatus capture_status() {
        if (!owner() || consumed_ || !capture_) return WordDispatchStatus::rejected;
        if (!capture_->completed) return WordDispatchStatus::pending;
        if (!capture_->captured || !capture_->captured->alive()) { cancel(); return WordDispatchStatus::rejected; }
        return WordDispatchStatus::captured;
    }
    uint64_t capture_id() const { return owner() && !consumed_ ? id_ : 0; }
    bool selected_text(uint64_t id, std::wstring& text) {
        text.clear();
        if (!owner() || !id || id != id_ || capture_status() != WordDispatchStatus::captured) return false;
        text = capture_->captured->source(); return true;
    }
    bool begin_accept(uint64_t id, std::wstring reviewed, std::chrono::steady_clock::time_point deadline) {
        if (!owner() || pending() || !id || id != id_ || capture_status() != WordDispatchStatus::captured ||
            std::chrono::steady_clock::now() >= deadline) return false;
        consumed_ = true;
        apply_.Attach(new WordEditSession(capture_->captured, std::move(reviewed), lifetime_, deadline));
        if (!request(apply_.Get(), TF_ES_READWRITE)) { cancel(); apply_.Reset(); return false; }
        return true;
    }
    WordDispatchStatus accept_status() const {
        if (!owner() || !apply_) return WordDispatchStatus::rejected;
        if (!apply_->completed) return WordDispatchStatus::pending;
        if (apply_->result == ApplyResult::applied) return WordDispatchStatus::applied;
        if (apply_->result == ApplyResult::mutation_unconfirmed) return WordDispatchStatus::mutation_unconfirmed;
        return WordDispatchStatus::rejected;
    }
    unsigned mutations() const { return apply_ ? apply_->mutations : 0; }
    // Runtime must observe close()==true before retiring this actor. Destruction
    // alone never qualifies cleanup; retained COM callbacks still hold lifetime.
    ~WordTargetDispatcher() { close(); }
};
}

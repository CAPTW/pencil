#pragma once
#include "word_edit_session.hpp"
#include "word_events.hpp"
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
    std::shared_ptr<WordEventBinding> events_;
    ComPtr<WordEditSession> capture_, apply_;
    uint64_t id_ = 0, next_ = 1;
    uint64_t revoked_ = 0;
    bool transitioning_ = false;
    struct Transition {
        bool& active;
        explicit Transition(bool& value):active(value){active=true;}
        ~Transition(){active=false;}
    };
    bool consumed_ = true;
    bool owner() const { return GetCurrentThreadId() == thread_; }
    bool request(WordEditSession* session, DWORD flags) {
        HRESULT disposition = E_PENDING;
        const auto hr = binding_->state->context->RequestEditSession(client_, session, TF_ES_ASYNC | flags, &disposition);
        if (session->completed || session->called) return true;
        if (FAILED(hr) || disposition != TF_S_ASYNC) { session->cancel(); return false; }
        return true;
    }
    WordDispatchStatus observe_capture() {
        if (consumed_ || !capture_) return WordDispatchStatus::rejected;
        if (!capture_->completed) return WordDispatchStatus::pending;
        const auto epoch = revoked_;
        auto lease = capture_->captured;
        if (!lease || !lease->alive() || epoch != revoked_ || consumed_) {
            cancel(); return WordDispatchStatus::rejected;
        }
        return WordDispatchStatus::captured;
    }
public:
    WordTargetDispatcher(TfClientId client, std::shared_ptr<void> callbackLifetime)
        : client_(client), lifetime_(std::move(callbackLifetime)) {}
    WordTargetDispatcher(const WordTargetDispatcher&) = delete;
    WordTargetDispatcher& operator=(const WordTargetDispatcher&) = delete;
    bool pending() const {
        return transitioning_ || (capture_ && !capture_->completed) || (apply_ && !apply_->completed);
    }
    void cancel() {
        if (!owner()) return;
        if(revoked_ != (std::numeric_limits<uint64_t>::max)())++revoked_;
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
        Transition transition(transitioning_);
        const bool eventsClosed = !events_ || events_->close();
        const bool contextClosed = !binding_ || binding_->close();
        if (!eventsClosed || !contextClosed) return false; // Retain unresolved cookies.
        apply_.Reset(); capture_.Reset(); events_.reset(); binding_.reset();
        return true;
    }
    bool begin_capture(ITfContext* context, HWND field, HWND root, std::shared_ptr<WordNativeWindowPort> port,
        bool explicitOptIn, std::function<bool()> authority, std::chrono::steady_clock::time_point deadline, ULONGLONG deadlineTicks=MAXULONGLONG) {
        if (!owner() || !lifetime_ || !context || !port || !explicitOptIn || !authority ||
            GetTickCount64() >= deadlineTicks || std::chrono::steady_clock::now() >= deadline || next_ == (std::numeric_limits<uint64_t>::max)() || pending()) return false;
        if (!close()) return false;
        Transition transition(transitioning_);
        const auto epoch=revoked_;
        if(epoch==(std::numeric_limits<uint64_t>::max)() || !authority() || epoch!=revoked_)return false;
        binding_ = std::make_unique<WordTsfRevisionBinding>(context, field, root, std::move(port), true, lifetime_, std::move(authority));
        if (epoch!=revoked_) {binding_->disable();return false;}
        if (!binding_->state->enabled) return false;
        const auto remaining=std::chrono::duration_cast<std::chrono::milliseconds>(deadline-std::chrono::steady_clock::now()).count();
        if(remaining<=0)return false;
        auto& state=*binding_->state;
        WordObservationGuard observation{state.thread,(std::min)(deadlineTicks,GetTickCount64()+static_cast<ULONGLONG>((std::min)(remaining,30000LL))),state.revision.load(),state.revision,state.enabled};
        auto target=WordTargetIdentity::capture(state.port,field,root,observation);
        if(!target)return false;
        events_=std::make_shared<WordEventBinding>();
        if(!events_->attach(binding_->state,*target,observation,lifetime_))return false;
        if(epoch!=revoked_){binding_->disable();return false;}
        std::weak_ptr<WordEventBinding> weakEvents=events_;
        state.application_matches=[weakEvents](IDispatch* app){auto events=weakEvents.lock();return events && events->matches(app);};
        capture_.Attach(new WordEditSession(binding_->state, lifetime_, deadline, deadlineTicks));
        id_ = next_++; consumed_ = false;
        if (!request(capture_.Get(), TF_ES_READ)) { cancel(); capture_.Reset(); return false; }
        return true;
    }
    WordDispatchStatus capture_status() {
        if (!owner() || transitioning_) return WordDispatchStatus::rejected;
        Transition transition(transitioning_);
        return observe_capture();
    }
    uint64_t capture_id() const { return owner() && !consumed_ ? id_ : 0; }
    bool selected_text(uint64_t id, std::wstring& text) {
        text.clear();
        if (!owner() || transitioning_ || !id || id != id_) return false;
        Transition transition(transitioning_);
        if (observe_capture() != WordDispatchStatus::captured || id != id_) return false;
        text = capture_->captured->source(); return true;
    }
    bool begin_accept(uint64_t id, std::wstring reviewed, std::chrono::steady_clock::time_point deadline, ULONGLONG deadlineTicks=MAXULONGLONG) {
        if (!owner() || pending() || !id || id != id_ ||
            GetTickCount64() >= deadlineTicks || std::chrono::steady_clock::now() >= deadline) return false;
        Transition transition(transitioning_);
        const auto epoch=revoked_;
        if(observe_capture()!=WordDispatchStatus::captured || epoch!=revoked_ || id!=id_)return false;
        consumed_ = true;
        apply_.Attach(new WordEditSession(capture_->captured, std::move(reviewed), lifetime_, deadline, deadlineTicks));
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

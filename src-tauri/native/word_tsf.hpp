#pragma once
// Dormant application adapter: target-side Word metadata + actual TSF selection.
// Real Word provider qualification/deployment remains required before admission.
#include "word_target.hpp"
#include "tsf_selected_range.hpp"
#include "tsf_subscription.hpp"

namespace grammar {
struct WordRequestGate {
    std::atomic<bool> valid{true};
    const std::chrono::steady_clock::time_point deadline;
    explicit WordRequestGate(std::chrono::steady_clock::time_point until) : deadline(until) {}
    bool alive() const { return valid.load() && std::chrono::steady_clock::now() < deadline; }
};
struct WordTsfState {
    const DWORD thread = GetCurrentThreadId();
    ComPtr<ITfContext> context;
    HWND field = nullptr, root = nullptr;
    std::shared_ptr<WordNativeWindowPort> port;
    std::atomic<bool> enabled{false};
    std::atomic<uint64_t> revision{0};
    std::weak_ptr<SelectedRangeGrant> outstanding;
    std::function<bool()> authority;
    void invalidate() {
        ++revision;
        if (auto grant = outstanding.lock()) grant->valid = false;
        outstanding.reset();
    }
    bool topology(ITfContext* current, TfEditCookie cookie) const {
        if (GetCurrentThreadId() != thread || !enabled || !context || !authority || !authority() ||
            !selected_range_detail::same(context.Get(), current)) return false;
        ComPtr<ITfContextView> view; HWND window = nullptr;
        ComPtr<ITfDocumentMgr> document; ComPtr<ITfContext> top;
        ComPtr<ITfRange> selected;
        return SUCCEEDED(context->GetActiveView(&view)) && view &&
            SUCCEEDED(view->GetWnd(&window)) && window == field &&
            SUCCEEDED(context->GetDocumentMgr(&document)) && document &&
            SUCCEEDED(document->GetTop(&top)) && selected_range_detail::same(context.Get(), top.Get()) &&
            selected_range_detail::selection(context.Get(), cookie, selected) && authority();
    }
};
class WordTsfRevisionSink final : public ITfTextEditSink {
    LONG refs_ = 1;
    std::weak_ptr<WordTsfState> state_;
    std::shared_ptr<void> callback_lifetime_;
public:
    WordTsfRevisionSink(std::weak_ptr<WordTsfState> state, std::shared_ptr<void> lifetime)
        : state_(std::move(state)), callback_lifetime_(std::move(lifetime)) {}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id, void** out) override {
        if (!out) return E_POINTER; *out = nullptr;
        if (id == IID_IUnknown || id == IID_ITfTextEditSink) { *out = static_cast<ITfTextEditSink*>(this); AddRef(); return S_OK; }
        return E_NOINTERFACE;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return InterlockedIncrement(&refs_); }
    ULONG STDMETHODCALLTYPE Release() override { const auto n = InterlockedDecrement(&refs_); if (!n) delete this; return n; }
    HRESULT STDMETHODCALLTYPE OnEndEdit(ITfContext*, TfEditCookie, ITfEditRecord*) override {
        if (auto state = state_.lock()) state->invalidate();
        return S_OK;
    }
};
// All construction/use/destruction on target STA, retained until callbacks end.
class WordTsfRevisionBinding final {
    TsfSinkSubscription subscription_;
public:
    const std::shared_ptr<WordTsfState> state = std::make_shared<WordTsfState>();
    WordTsfRevisionBinding(ITfContext* context, HWND field, HWND root,
        std::shared_ptr<WordNativeWindowPort> port, bool explicitOptIn, std::shared_ptr<void> callbackLifetime,
        std::function<bool()> authority) {
        state->context = context; state->field = field; state->root = root; state->port = std::move(port);
        state->authority = std::move(authority);
        if (!explicitOptIn || !context || !state->port || !callbackLifetime || !state->authority ||
            !state->port->eligible(field, root, state->thread)) return;
        ComPtr<WordTsfRevisionSink> sink; sink.Attach(new WordTsfRevisionSink(state, std::move(callbackLifetime)));
        ComPtr<ITfSource> source;
        state->enabled = SUCCEEDED(context->QueryInterface(IID_PPV_ARGS(&source))) &&
            subscription_.attach(source.Get(), IID_ITfTextEditSink, sink.Get());
    }
    WordTsfRevisionBinding(const WordTsfRevisionBinding&) = delete;
    WordTsfRevisionBinding& operator=(const WordTsfRevisionBinding&) = delete;
    void disable() { state->enabled = false; state->invalidate(); }
    bool close() {
        if (GetCurrentThreadId() != state->thread) return false;
        disable();
        return subscription_.close();
    }
    bool disconnected() const { return subscription_.disconnected(); }
    ~WordTsfRevisionBinding() {
        disable();
        // subscription destructor is best effort; only an observed close()
        // result plus expired callback owners qualifies runtime retirement.
    }
};

class WordCorrelationLease final : public std::enable_shared_from_this<WordCorrelationLease> {
    std::weak_ptr<WordTsfState> state_;
    std::unique_ptr<WordTargetIdentity> target_;
    const uint64_t revision_;
    const ULONGLONG deadline_;
    std::unique_ptr<TsfSelectedSnapshot> snapshot_;
    std::shared_ptr<SelectedRangeGrant> grant_;
    std::shared_ptr<WordRequestGate> operation_;
    WordCorrelationLease(const std::shared_ptr<WordTsfState>& state)
        : state_(state), revision_(state->revision), deadline_(GetTickCount64() + 30000) {}
    WordObservationGuard guard(const WordTsfState& state) const {
        return {state.thread, deadline_, revision_, state.revision, state.enabled};
    }
    bool metadata(ITfContext* current, TfEditCookie cookie) const {
        auto state = state_.lock();
        return (!operation_ || operation_->alive()) && state && guard(*state).current() && state->topology(current, cookie) && target_ &&
            target_->matches(guard(*state)) && guard(*state).current() && (!operation_ || operation_->alive());
    }
    // Called ONLY after a real context/cookie/selection and privacy metadata
    // check. This selected Text getter is separate from metadata-only helpers.
    bool word_source_is(const std::wstring& expected, ITfContext* current, TfEditCookie cookie) const {
        auto state = state_.lock();
        if (!state || !metadata(current, cookie) || !selected_range_detail::plain(expected)) return false;
        const auto observation = guard(*state);
        ComPtr<IDispatch> selected, range;
        if (!word_selection_detail::object(target_->window_.Get(), L"Selection", selected, observation) ||
            !word_selection_detail::same(selected.Get(), target_->selection_->metadata_.selection.Get()) ||
            !word_selection_detail::object(selected.Get(), L"Range", range, observation) ||
            !metadata(current, cookie)) return false;
        ComPtr<IDispatch> document;
        LONG start = -1, end = -1, story = -1;
        const auto& captured = target_->selection_->metadata_;
        if (!word_selection_detail::object(range.Get(), L"Document", document, observation) ||
            !word_selection_detail::same(document.Get(), captured.document.Get()) ||
            !word_selection_detail::number(range.Get(), L"Start", start, observation) || start != captured.start ||
            !word_selection_detail::number(range.Get(), L"End", end, observation) || end != captured.end ||
            !word_selection_detail::number(range.Get(), L"StoryType", story, observation) || story != 1 ||
            !metadata(current, cookie)) return false;
        word_privacy_detail::Value text;
        if (!word_privacy_detail::get(range.Get(), L"Text", text) || text.value.vt != VT_BSTR ||
            !text.value.bstrVal || !observation.current() || SysStringLen(text.value.bstrVal) != expected.size() ||
            std::wstring(text.value.bstrVal, SysStringLen(text.value.bstrVal)) != expected) return false;
        return metadata(current, cookie);
    }
public:
    static std::shared_ptr<WordCorrelationLease> capture(const std::shared_ptr<WordTsfState>& state,
        TfEditCookie cookie, unsigned& reads, std::shared_ptr<WordRequestGate> operation = {}) {
        if ((operation && !operation->alive()) || !state || GetCurrentThreadId() != state->thread || !state->enabled ||
            !state->topology(state->context.Get(), cookie)) return {};
        state->invalidate(); // A newer capture revokes the previous lease.
        auto lease = std::shared_ptr<WordCorrelationLease>(new WordCorrelationLease(state));
        lease->operation_ = std::move(operation);
        lease->target_ = WordTargetIdentity::capture(state->port, state->field, state->root, lease->guard(*state));
        if (!lease->target_) return {};
        auto grant = std::make_shared<SelectedRangeGrant>();
        grant->field_eligible = true; grant->revision = lease->revision_;
        const auto now = GetTickCount64();
        grant->deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(
            lease->deadline_ > now ? lease->deadline_ - now : 0);
        std::weak_ptr<WordCorrelationLease> weak = lease;
        grant->verify_binding = [weak](ITfContext* context, TfEditCookie currentCookie) {
            auto value = weak.lock();
            if (!value || !value->metadata(context, currentCookie)) return false;
            return !value->snapshot_ || value->word_source_is(value->snapshot_->source(), context, currentCookie);
        };
        lease->grant_ = grant; state->outstanding = grant;
        lease->snapshot_ = TsfSelectedSnapshot::capture(state->context.Get(), cookie, lease->revision_, grant, reads);
        if (!lease->snapshot_ || !lease->word_source_is(lease->snapshot_->source(), state->context.Get(), cookie)) return {};
        lease->operation_.reset(); // Capture deadline is not the review lease.
        return lease;
    }
    const std::wstring& source() const { return snapshot_->source(); }
    ApplyResult accept(TfEditCookie cookie, const std::wstring& reviewed, unsigned& reads, unsigned& mutations,
        std::shared_ptr<WordRequestGate> operation = {}) {
        // Caller owns the actual write session and explicit reviewed Accept.
        if (operation_ || (operation && !operation->alive())) { cancel(); return ApplyResult::rejected; }
        operation_ = std::move(operation);
        return snapshot_ ? snapshot_->apply(cookie, reviewed, reads, mutations) : ApplyResult::rejected;
    }
    void cancel() { if (grant_) grant_->valid = false; }
    ~WordCorrelationLease() { cancel(); }
};
}

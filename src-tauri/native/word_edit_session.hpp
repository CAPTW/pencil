#pragma once
#include "word_tsf.hpp"

namespace grammar {
// One immutable request. Owner retains the COM session until completed; cancel
// revokes authority, not the callback. Missing lifetime ownership refuses work.
class WordEditSession final : public ITfEditSession {
    LONG refs_ = 1;
    const DWORD thread_ = GetCurrentThreadId();
    std::shared_ptr<void> callback_lifetime_;
    std::weak_ptr<WordTsfState> state_;
    std::shared_ptr<WordCorrelationLease> input_;
    const bool applying_;
    std::wstring replacement_;
    const std::shared_ptr<WordRequestGate> gate_;
public:
    bool called = false, completed = false;
    ApplyResult result = ApplyResult::pending;
    unsigned text_reads = 0, mutations = 0;
    std::shared_ptr<WordCorrelationLease> captured;
    WordEditSession(const std::shared_ptr<WordTsfState>& state, std::shared_ptr<void> lifetime,
        std::chrono::steady_clock::time_point deadline, ULONGLONG deadlineTicks=MAXULONGLONG)
        : callback_lifetime_(std::move(lifetime)), state_(state), applying_(false), gate_(std::make_shared<WordRequestGate>(deadline, deadlineTicks)) {}
    WordEditSession(std::shared_ptr<WordCorrelationLease> input, std::wstring replacement,
        std::shared_ptr<void> lifetime, std::chrono::steady_clock::time_point deadline, ULONGLONG deadlineTicks=MAXULONGLONG)
        : callback_lifetime_(std::move(lifetime)), input_(std::move(input)), applying_(true),
          replacement_(std::move(replacement)), gate_(std::make_shared<WordRequestGate>(deadline, deadlineTicks)) {}
    void cancel() {
        gate_->valid = false;
        if (GetCurrentThreadId() == thread_) {
            if (input_) input_->cancel();
            if (captured) captured->cancel();
        }
    }
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id, void** out) override {
        if (!out) return E_POINTER; *out = nullptr;
        if (id == IID_IUnknown || id == IID_ITfEditSession) { *out = static_cast<ITfEditSession*>(this); AddRef(); return S_OK; }
        return E_NOINTERFACE;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return InterlockedIncrement(&refs_); }
    ULONG STDMETHODCALLTYPE Release() override { const auto n = InterlockedDecrement(&refs_); if (!n) delete this; return n; }
    HRESULT STDMETHODCALLTYPE DoEditSession(TfEditCookie cookie) override {
        if (called || GetCurrentThreadId() != thread_) return E_UNEXPECTED;
        called = true; result = ApplyResult::rejected;
        struct Completion {
            WordEditSession* session;
            ~Completion() {
                auto& text = session->replacement_;
                if (!text.empty()) SecureZeroMemory(text.data(), text.size() * sizeof(wchar_t));
                text.clear(); session->completed = true;
            }
        } completion{this};
        if (!callback_lifetime_ || !gate_->alive()) { if (input_) input_->cancel(); return S_FALSE; }
        try {
            if (applying_) {
                if (input_) result = input_->accept(cookie, replacement_, text_reads, mutations, gate_);
                return result == ApplyResult::applied ? S_OK : S_FALSE;
            }
            auto state = state_.lock();
            if (!state) return S_FALSE;
            captured = WordCorrelationLease::capture(state, cookie, text_reads, gate_);
            if (!gate_->alive()) { if (captured) captured->cancel(); captured.reset(); }
            return captured ? S_OK : S_FALSE;
        } catch (...) {
            if (input_) input_->cancel();
            if (captured) captured->cancel();
            captured.reset();
            result = mutations ? ApplyResult::mutation_unconfirmed : ApplyResult::rejected;
            return E_FAIL;
        }
    }
    ~WordEditSession() { if (!replacement_.empty()) SecureZeroMemory(replacement_.data(), replacement_.size() * sizeof(wchar_t)); }
};
}

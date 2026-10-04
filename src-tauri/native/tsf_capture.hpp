#pragma once
// Experimental target-thread capture. No registration or cross-process bridge.
#include "tsf_apply.hpp"
#include <inputscope.h>
#include <richedit.h>
#include <tom.h>
#include <commctrl.h>

namespace grammar {
// SDK GUID_PROP_INPUTSCOPE; some SDK uuid.lib versions omit its definition.
inline constexpr GUID input_scope_property = {0x1713dd5a, 0x68e7, 0x4a5b, {0x9a, 0xf6, 0x59, 0x2a, 0x59, 0x5c, 0x77, 0x8d}};
inline bool same_com(IUnknown* a, IUnknown* b) {
    if (!a || !b) return false;
    ComPtr<IUnknown> left, right;
    return SUCCEEDED(a->QueryInterface(IID_PPV_ARGS(&left))) &&
        SUCCEEDED(b->QueryInterface(IID_PPV_ARGS(&right))) && left.Get() == right.Get();
}

struct TsfBindingState {
    ComPtr<ITfContext> context;
    const HWND window;
    const DWORD thread = GetCurrentThreadId();
    bool enabled = false;
    unsigned long long revision = 0;
    unsigned notifications = 0;
    unsigned eligibility_transitions = 0;
    mutable unsigned eligibility_stage = 0;
    mutable unsigned scope_type = 0;
    std::weak_ptr<ApplyGrant> outstanding;
    TsfBindingState(ITfContext* value, HWND hwnd) : context(value), window(hwnd) {}
    void invalidate() {
        ++revision;
        if (auto grant = outstanding.lock()) { grant->valid = false; grant->revision = revision; }
        outstanding.reset();
    }
    bool eligible(ITfContext* current, TfEditCookie cookie) const {
        eligibility_stage = 1;
        if (!enabled || GetCurrentThreadId() != thread || !same_com(context.Get(), current)) return false;
        DWORD process = 0;
        eligibility_stage = 2;
        if (!IsWindow(window) || GetWindowThreadProcessId(window, &process) != thread ||
            process != GetCurrentProcessId() || !IsWindowVisible(window) || !IsWindowEnabled(window)) return false;
        WCHAR name[64]{};
        eligibility_stage = 3;
        if (!GetClassNameW(window, name, 64) || _wcsicmp(name, L"RICHEDIT50W") != 0 ||
            (GetWindowLongPtrW(window, GWL_STYLE) & (ES_PASSWORD | ES_READONLY)) ||
            SendMessageW(window, EM_GETPASSWORDCHAR, 0, 0)) return false;
        // Inspect formatting metadata for the whole story without selecting or
        // reading text. Protected/hidden/mixed/unknown formatting is ineligible.
        ComPtr<IUnknown> ole; ComPtr<ITextDocument> richDocument;
        ComPtr<ITextRange> story; ComPtr<ITextFont> font;
        LONG expanded = 0, protectedValue = tomUndefined, hiddenValue = tomUndefined;
        eligibility_stage = 11;
        if (!SendMessageW(window, EM_GETOLEINTERFACE, 0, reinterpret_cast<LPARAM>(ole.GetAddressOf())) ||
            !ole || FAILED(ole.As(&richDocument)) || FAILED(richDocument->Range(0, 0, &story)) ||
            FAILED(story->Expand(tomStory, &expanded)) || FAILED(story->GetFont(&font)) ||
            FAILED(font->GetProtected(&protectedValue)) || protectedValue != tomFalse ||
            FAILED(font->GetHidden(&hiddenValue)) || hiddenValue != tomFalse) return false;
        ComPtr<ITfContextView> view; HWND actual = nullptr;
        eligibility_stage = 4;
        if (FAILED(context->GetActiveView(&view)) || FAILED(view->GetWnd(&actual)) || actual != window) return false;
        ComPtr<ITfDocumentMgr> document; ComPtr<ITfContext> top;
        eligibility_stage = 5;
        if (FAILED(context->GetDocumentMgr(&document)) || FAILED(document->GetTop(&top)) ||
            !same_com(top.Get(), context.Get())) return false;
        TF_STATUS status{};
        eligibility_stage = 6;
        if (FAILED(context->GetStatus(&status)) || (status.dwDynamicFlags & TF_SD_READONLY)) return false;
        ComPtr<ITfContextComposition> composition; ComPtr<IEnumITfCompositionView> enumeration;
        eligibility_stage = 7;
        ComPtr<ITfCompositionView> item; ULONG count = 0;
        if (FAILED(context.As(&composition)) || FAILED(composition->EnumCompositions(&enumeration)) ||
            FAILED(enumeration->Next(1, &item, &count)) || count) return false;
        // App-wide input scope must explicitly resolve to plain text. Unknown,
        // mixed, password and every other scope fail before GetText.
        ComPtr<ITfReadOnlyProperty> property; ComPtr<ITfRange> range, end;
        eligibility_stage = 8;
        if (context->GetAppProperty(input_scope_property, &property) != S_OK || !property ||
            FAILED(context->GetStart(cookie, &range)) || FAILED(context->GetEnd(cookie, &end)) ||
            FAILED(range->ShiftEndToRange(cookie, end.Get(), TF_ANCHOR_END))) return false;
        VARIANT value; VariantInit(&value);
        HRESULT hr = property->GetValue(cookie, range.Get(), &value);
        eligibility_stage = 9;
        scope_type = value.vt;
        ComPtr<ITfInputScope> scope;
        bool allowed = false;
        if (SUCCEEDED(hr) && value.vt == VT_UNKNOWN && value.punkVal &&
            SUCCEEDED(value.punkVal->QueryInterface(IID_PPV_ARGS(&scope)))) {
            InputScope* scopes = nullptr; UINT size = 0;
            hr = scope->GetInputScopes(&scopes, &size);
            allowed = SUCCEEDED(hr) && scopes && size > 0 && size <= 32;
            if (allowed) for (UINT i = 0; i < size; ++i)
                if (scopes[i] != IS_DEFAULT && scopes[i] != IS_TEXT) allowed = false;
            CoTaskMemFree(scopes);
        }
        VariantClear(&value);
        if (allowed) eligibility_stage = 10;
        return allowed;
    }
};

class TsfRevisionSink final : public ITfTextEditSink {
    LONG refs_ = 1;
    std::weak_ptr<TsfBindingState> state_;
public:
    explicit TsfRevisionSink(std::weak_ptr<TsfBindingState> state) : state_(std::move(state)) {}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id, void** out) override {
        if (!out) return E_POINTER;
        *out = nullptr;
        if (id != IID_IUnknown && id != IID_ITfTextEditSink) return E_NOINTERFACE;
        *out = static_cast<ITfTextEditSink*>(this); AddRef(); return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return InterlockedIncrement(&refs_); }
    ULONG STDMETHODCALLTYPE Release() override { auto n = InterlockedDecrement(&refs_); if (!n) delete this; return n; }
    HRESULT STDMETHODCALLTYPE OnEndEdit(ITfContext*, TfEditCookie, ITfEditRecord*) override {
        auto state = state_.lock();
        if (!state) return S_OK;
        ++state->notifications;
        // Every completed write session advances the revision, including app
        // properties (input scope) and no-op writes. Text-only update records
        // miss privacy changes that are subsequently reverted. Read sessions do
        // not trigger this sink. No content reads or automatic rewriting here.
        state->invalidate();
        return S_OK;
    }
};

// Owns the subscription. The sink is weak to avoid a context/sink/state cycle.
// Construct, use and destroy on the target STA. At most one outstanding grant.
class TsfDocumentBinding final {
    ComPtr<ITfSource> source_;
    DWORD cookie_ = TF_INVALID_COOKIE;
    bool subclass_installed_ = false;
    static LRESULT CALLBACK window_proc(HWND hwnd, UINT message, WPARAM wparam, LPARAM lparam,
                                         UINT_PTR id, DWORD_PTR data) {
        auto state = reinterpret_cast<TsfBindingState*>(data);
        // RichEdit may synchronously deliver an already queued TSF write from
        // inside EM_SETPASSWORDCHAR, BEFORE updating the password style. Revoke
        // at message entry, before forwarding; never inspect keys or text.
        if (message == EM_SETPASSWORDCHAR || message == EM_SETREADONLY ||
            message == EM_SETCHARFORMAT || message == WM_STYLECHANGING || message == WM_NCDESTROY) {
            ++state->eligibility_transitions;
            state->invalidate();
        }
        if (message == WM_NCDESTROY) {
            state->enabled = false;
            RemoveWindowSubclass(hwnd, window_proc, id);
        }
        return DefSubclassProc(hwnd, message, wparam, lparam);
    }
public:
    const std::shared_ptr<TsfBindingState> state;
    TsfDocumentBinding(ITfContext* context, HWND window, bool explicitlyEnabled)
        : state(std::make_shared<TsfBindingState>(context, window)) {
        if (!explicitlyEnabled) return;
        DWORD process = 0; ComPtr<ITfContextView> view; HWND actual = nullptr;
        if (!context || GetWindowThreadProcessId(window, &process) != state->thread ||
            process != GetCurrentProcessId() || FAILED(context->GetActiveView(&view)) || !view ||
            FAILED(view->GetWnd(&actual)) || actual != window) return;
        subclass_installed_ = SetWindowSubclass(window, window_proc, reinterpret_cast<UINT_PTR>(this),
            reinterpret_cast<DWORD_PTR>(state.get())) != FALSE;
        if (!subclass_installed_) return;
        ComPtr<TsfRevisionSink> sink; sink.Attach(new TsfRevisionSink(state));
        state->enabled = context && SUCCEEDED(context->QueryInterface(IID_PPV_ARGS(&source_))) &&
            SUCCEEDED(source_->AdviseSink(IID_ITfTextEditSink, sink.Get(), &cookie_));
    }
    TsfDocumentBinding(const TsfDocumentBinding&) = delete;
    TsfDocumentBinding& operator=(const TsfDocumentBinding&) = delete;
    void disable() { state->enabled = false; state->invalidate(); }
    ~TsfDocumentBinding() {
        disable();
        if (subclass_installed_) RemoveWindowSubclass(state->window, window_proc, reinterpret_cast<UINT_PTR>(this));
        if (source_ && cookie_ != TF_INVALID_COOKIE) source_->UnadviseSink(cookie_);
    }
};

class TsfCaptureSession final : public ITfEditSession {
    LONG refs_ = 1;
    std::weak_ptr<TsfBindingState> state_;
    const unsigned long long revision_;
    const std::chrono::steady_clock::time_point deadline_ = std::chrono::steady_clock::now() + std::chrono::seconds(5);
public:
    bool called = false, captured = false;
    unsigned text_reads = 0;
    std::wstring source;
    LONG start = -1, end = -1;
    std::shared_ptr<ApplyGrant> grant;
    explicit TsfCaptureSession(const std::shared_ptr<TsfBindingState>& state)
        : state_(state), revision_(state->revision) {}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id, void** out) override {
        if (!out) return E_POINTER;
        *out = nullptr;
        if (id != IID_IUnknown && id != IID_ITfEditSession) return E_NOINTERFACE;
        *out = static_cast<ITfEditSession*>(this); AddRef(); return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return InterlockedIncrement(&refs_); }
    ULONG STDMETHODCALLTYPE Release() override { auto n = InterlockedDecrement(&refs_); if (!n) delete this; return n; }
    HRESULT STDMETHODCALLTYPE DoEditSession(TfEditCookie cookie) override try {
        if (called) return E_UNEXPECTED;
        called = true;
        auto state = state_.lock();
        if (!state || std::chrono::steady_clock::now() >= deadline_ ||
            state->revision != revision_ || !state->eligible(state->context.Get(), cookie)) return S_FALSE;
        TF_SELECTION selected{}; ULONG count = 0;
        if (FAILED(state->context->GetSelection(cookie, TF_DEFAULT_SELECTION, 1, &selected, &count)) || count != 1) return S_FALSE;
        ComPtr<ITfRange> selection; selection.Attach(selected.range);
        ComPtr<ITfRangeACP> acp; LONG length = 0;
        if (FAILED(selection.As(&acp)) || FAILED(acp->GetExtent(&start, &length)) ||
            start < 0 || length <= 0 || start > 65534 || length > 65534 - start) return S_FALSE;
        end = start + length;
        ComPtr<ITfRange> range, tail;
        if (FAILED(state->context->GetStart(cookie, &range)) || FAILED(state->context->GetEnd(cookie, &tail)) ||
            FAILED(range->ShiftEndToRange(cookie, tail.Get(), TF_ANCHOR_END))) return S_FALSE;
        std::vector<WCHAR> buffer(65535);
        ++text_reads;
        if (FAILED(range->GetText(cookie, 0, buffer.data(), static_cast<ULONG>(buffer.size()), &count)) ||
            count >= buffer.size() || static_cast<ULONG>(end) > count) return S_FALSE;
        std::wstring text(buffer.data(), count);
        if (text.find(L'\0') != std::wstring::npos || !state->eligible(state->context.Get(), cookie) ||
            state->revision != revision_ || std::chrono::steady_clock::now() >= deadline_) return S_FALSE;
        state->invalidate(); // Supersede every earlier capture before issuing.
        auto issued = std::make_shared<ApplyGrant>();
        const auto revision = state->revision;
        issued->revision = revision; issued->field_eligible = true;
        issued->deadline = std::chrono::steady_clock::now() + std::chrono::seconds(30);
        issued->verify_binding = [weak = state_, revision](ITfContext* context, TfEditCookie editCookie) {
            auto current = weak.lock();
            return current && current->revision == revision && current->eligible(context, editCookie);
        };
        state->outstanding = issued;
        source = std::move(text); grant = std::move(issued); captured = true;
        return S_OK;
    } catch (const std::bad_alloc&) { return E_OUTOFMEMORY; }
      catch (...) { return E_FAIL; }
};
}

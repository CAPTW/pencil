#pragma once
// Dormant Word target-side acquisition. Never enumerate the ROT or start Office.
// The authenticated target service must qualify/deploy the adapter separately.
#include "word_selection.hpp"
#include <oleacc.h>
#include <string>

namespace grammar {
struct WordNativeWindowPort {
    virtual ~WordNativeWindowPort() = default;
    virtual bool eligible(HWND field, HWND root, DWORD ownerThread) = 0;
    virtual HRESULT native_window(HWND field, IDispatch** result) = 0;
};
class WordNativeWindowWin32 final : public WordNativeWindowPort {
public:
    bool eligible(HWND field, HWND root, DWORD ownerThread) override {
        if (GetCurrentThreadId() != ownerThread || !field || !root ||
            !IsWindow(field) || !IsWindow(root) || GetAncestor(field, GA_ROOT) != root ||
            !IsWindowVisible(field) || !IsWindowEnabled(field)) return false;
        DWORD fieldProcess = 0, rootProcess = 0;
        if (GetWindowThreadProcessId(field, &fieldProcess) != ownerThread ||
            GetWindowThreadProcessId(root, &rootProcess) != ownerThread ||
            fieldProcess != GetCurrentProcessId() || rootProcess != fieldProcess) return false;
        WCHAR fieldClass[64]{}, rootClass[64]{}, image[32768]{};
        if (!GetClassNameW(field, fieldClass, 64) || wcscmp(fieldClass, L"_WwG") != 0 ||
            !GetClassNameW(root, rootClass, 64) || wcscmp(rootClass, L"OpusApp") != 0) return false;
        const auto count = GetModuleFileNameW(nullptr, image, 32768);
        if (!count || count >= 32768) return false;
        const auto* name = wcsrchr(image, L'\\');
        // This is an admission prerequisite, NOT executable authentication.
        return name && _wcsicmp(name + 1, L"WINWORD.EXE") == 0;
    }
    HRESULT native_window(HWND field, IDispatch** result) override {
        if (!result) return E_POINTER;
        *result = nullptr;
        return AccessibleObjectFromWindow(field, static_cast<DWORD>(OBJID_NATIVEOM),
            IID_IDispatch, reinterpret_cast<void**>(result));
    }
};

class WordTargetIdentity final {
    friend class WordCorrelationLease;
    friend class WordEventBinding;
    std::shared_ptr<WordNativeWindowPort> port_;
    Microsoft::WRL::ComPtr<IDispatch> window_, application_;
    std::unique_ptr<WordSelectionIdentity> selection_;
    HWND field_, root_;
    DWORD thread_;
    WordTargetIdentity(std::shared_ptr<WordNativeWindowPort> port, HWND field, HWND root, DWORD thread)
        : port_(std::move(port)), field_(field), root_(root), thread_(thread) {}
    bool acquire(const WordObservationGuard& guard, Microsoft::WRL::ComPtr<IDispatch>& window,
                 Microsoft::WRL::ComPtr<IDispatch>& application) const {
        using namespace word_selection_detail;
        if (guard.thread != thread_ || !guard.current() || !port_ ||
            !port_->eligible(field_, root_, thread_) || !guard.current() ||
            port_->native_window(field_, window.GetAddressOf()) != S_OK || !window || !guard.current()) return false;
        LONG hwnd = 0;
        Microsoft::WRL::ComPtr<IDispatch> active;
        return number(window.Get(), L"Hwnd", hwnd, guard) &&
            hwnd == static_cast<LONG>(reinterpret_cast<INT_PTR>(root_)) &&
            object(window.Get(), L"Application", application, guard) &&
            object(application.Get(), L"ActiveWindow", active, guard) && same(active.Get(), window.Get()) &&
            guard.current() && port_->eligible(field_, root_, thread_) && guard.current();
    }
public:
    WordTargetIdentity(const WordTargetIdentity&) = delete;
    WordTargetIdentity& operator=(const WordTargetIdentity&) = delete;
    static std::unique_ptr<WordTargetIdentity> capture(std::shared_ptr<WordNativeWindowPort> port,
        HWND field, HWND root, const WordObservationGuard& guard) {
        auto target = std::unique_ptr<WordTargetIdentity>(new WordTargetIdentity(std::move(port), field, root, guard.thread));
        if (!target->acquire(guard, target->window_, target->application_)) return {};
        target->selection_ = WordSelectionIdentity::capture(target->application_.Get(), root, guard);
        if (!target->selection_ || !target->matches(guard)) return {};
        return target;
    }
    bool matches(const WordObservationGuard& guard) const {
        Microsoft::WRL::ComPtr<IDispatch> window, app;
        return acquire(guard, window, app) &&
            word_selection_detail::same(window.Get(), window_.Get()) &&
            word_selection_detail::same(app.Get(), application_.Get()) && selection_ &&
            selection_->matches(guard) && guard.current() &&
            port_->eligible(field_, root_, thread_) && guard.current();
    }
    // Metadata only: no application/selection pointer, text or mutation exposed.
    // Actual TSF view/context + document coupling and revision owner are still
    // required; neither this handle check nor executable basename grants trust.
};
}

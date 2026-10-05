#pragma once
// Dormant application-adapter transaction. No class admission or registration.
// Caller must authenticate the real context and provide privacy/revision authority
// on the owning thread. A synthetic binding callback is test authority only.
#include "tsf_apply.hpp"

namespace grammar {
struct SelectedRangeGrant final : ApplyGrant {
    std::atomic<bool> claimed{false};
};
namespace selected_range_detail {
inline bool same(IUnknown* a, IUnknown* b) {
    if (!a || !b) return false;
    ComPtr<IUnknown> x, y;
    return a->QueryInterface(IID_PPV_ARGS(&x)) == S_OK &&
        b->QueryInterface(IID_PPV_ARGS(&y)) == S_OK && x.Get() == y.Get();
}
inline bool plain(const std::wstring& text) {
    if (text.empty() || text.size() > 8192) return false;
    for (size_t i = 0; i < text.size(); ++i) {
        const auto c = text[i];
        if (!c || c == 0xfffc) return false;
        if (c >= 0xd800 && c <= 0xdbff) {
            if (++i == text.size() || text[i] < 0xdc00 || text[i] > 0xdfff) return false;
        } else if (c >= 0xdc00 && c <= 0xdfff) return false;
    }
    return true;
}
inline bool read(ITfRange* range, TfEditCookie cookie, std::wstring& text, unsigned& reads) {
    ComPtr<ITfRange> cursor;
    if (!range || FAILED(range->Clone(&cursor))) return false;
    text.clear();
    // MOVESTART on a clone makes progress without touching the selected anchors.
    // Never use IGNOREEND: even one lookahead unit stays inside the selection.
    for (unsigned attempt = 0; attempt < 8193; ++attempt) {
        BOOL empty = FALSE;
        if (FAILED(cursor->IsEmpty(cookie, &empty))) return false;
        if (empty) return plain(text);
        WCHAR buffer[256]; ULONG count = 0;
        ++reads;
        if (FAILED(cursor->GetText(cookie, TF_TF_MOVESTART, buffer, 256, &count)) ||
            !count || count > 256 || text.size() + count > 8192) return false;
        text.append(buffer, count);
    }
    return false;
}
inline bool equal(ITfRange* a, ITfRange* b, ITfContext* context, TfEditCookie cookie) {
    if (!a || !b || !context) return false;
    ComPtr<ITfContext> ac, bc;
    BOOL start = FALSE, end = FALSE;
    return SUCCEEDED(a->GetContext(&ac)) && SUCCEEDED(b->GetContext(&bc)) &&
        same(ac.Get(), context) && same(bc.Get(), context) &&
        SUCCEEDED(a->IsEqualStart(cookie, b, TF_ANCHOR_START, &start)) && start &&
        SUCCEEDED(a->IsEqualEnd(cookie, b, TF_ANCHOR_END, &end)) && end;
}
inline bool ready(ITfContext* context) {
    TF_STATUS status{};
    ComPtr<ITfContextComposition> composition;
    ComPtr<IEnumITfCompositionView> items;
    ComPtr<ITfCompositionView> item; ULONG fetched = 0;
    return context && SUCCEEDED(context->GetStatus(&status)) && !(status.dwDynamicFlags & TF_SD_READONLY) &&
        SUCCEEDED(context->QueryInterface(IID_PPV_ARGS(&composition))) &&
        SUCCEEDED(composition->EnumCompositions(&items)) &&
        SUCCEEDED(items->Next(1, &item, &fetched)) && !fetched;
}
inline bool selection(ITfContext* context, TfEditCookie cookie, ComPtr<ITfRange>& range) {
    TF_SELECTION selected{}; ULONG count = 0;
    const HRESULT hr = context->GetSelection(cookie, TF_DEFAULT_SELECTION, 1, &selected, &count);
    range.Attach(selected.range);
    BOOL empty = TRUE;
    return SUCCEEDED(hr) && count == 1 && range && !selected.style.fInterimChar &&
        SUCCEEDED(range->IsEmpty(cookie, &empty)) && !empty;
}
}

class TsfSelectedSnapshot final {
    ComPtr<ITfContext> context_;
    ComPtr<ITfRange> range_;
    std::wstring source_;
    const DWORD thread_;
    const unsigned long long revision_;
    std::shared_ptr<SelectedRangeGrant> grant_;
    bool consumed_ = false; // Access only on the owning STA, before callbacks.
    bool authority(TfEditCookie cookie, bool requireUnused) const {
        return GetCurrentThreadId() == thread_ && grant_ && (!requireUnused || grant_->valid.load()) &&
            grant_->field_eligible && grant_->revision.load() == revision_ &&
            std::chrono::steady_clock::now() < grant_->deadline && grant_->verify_binding &&
            grant_->verify_binding(context_.Get(), cookie) && grant_->revision.load() == revision_ &&
            grant_->valid.load() && grant_->field_eligible && std::chrono::steady_clock::now() < grant_->deadline;
    }
    TsfSelectedSnapshot(ITfContext* context, unsigned long long revision, std::shared_ptr<SelectedRangeGrant> grant)
        : context_(context), thread_(GetCurrentThreadId()), revision_(revision), grant_(std::move(grant)) {}
public:
    TsfSelectedSnapshot(const TsfSelectedSnapshot&) = delete;
    TsfSelectedSnapshot& operator=(const TsfSelectedSnapshot&) = delete;
    // Must be called inside an authenticated TSF read session. This method does
    // not obtain a lock, discover an app, read whole documents, or infer consent.
    static std::unique_ptr<TsfSelectedSnapshot> capture(ITfContext* context, TfEditCookie cookie,
        unsigned long long revision, std::shared_ptr<SelectedRangeGrant> grant, unsigned& reads) {
        if (!grant || grant->claimed.load()) return {};
        auto snapshot = std::unique_ptr<TsfSelectedSnapshot>(new TsfSelectedSnapshot(context, revision, std::move(grant)));
        ComPtr<ITfRange> selected;
        if (!snapshot->authority(cookie, true) || !selected_range_detail::ready(context) ||
            !selected_range_detail::selection(context, cookie, selected) ||
            FAILED(selected->Clone(&snapshot->range_)) ||
            !selected_range_detail::equal(selected.Get(), snapshot->range_.Get(), context, cookie) ||
            !snapshot->authority(cookie, true) ||
            !selected_range_detail::read(snapshot->range_.Get(), cookie, snapshot->source_, reads) ||
            !snapshot->authority(cookie, true)) return {};
        // A reentrant provider callback may move the selection while reading.
        ComPtr<ITfRange> current;
        if (!selected_range_detail::selection(context, cookie, current) ||
            !selected_range_detail::equal(current.Get(), snapshot->range_.Get(), context, cookie) ||
            !snapshot->authority(cookie, true)) return {};
        return snapshot;
    }
    const std::wstring& source() const { return source_; }
    // The request owner must revoke on RequestEditSession rejection or deadline;
    // an uninvoked callback is not a completed transaction. No editor access.
    void revoke() { if (grant_) grant_->valid = false; }
    // Must be called once inside a TSF WRITE session after explicit reviewed
    // Accept. A consumed grant is never revived, including on failure.
    ApplyResult apply(TfEditCookie cookie, const std::wstring& replacement, unsigned& reads, unsigned& mutations) {
        const auto before = mutations;
        try {
        if (GetCurrentThreadId() != thread_ || consumed_ || !grant_ || grant_->claimed.exchange(true)) return ApplyResult::rejected;
        consumed_ = true;
        struct Revoke { std::shared_ptr<ApplyGrant> grant; ~Revoke() { grant->valid = false; } } revoke{grant_};
        if (!context_ || !selected_range_detail::plain(replacement) || !authority(cookie, true) ||
            !selected_range_detail::ready(context_.Get())) return ApplyResult::rejected;
        ComPtr<ITfRange> current;
        std::wstring actual;
        if (!selected_range_detail::selection(context_.Get(), cookie, current) ||
            !selected_range_detail::equal(current.Get(), range_.Get(), context_.Get(), cookie) ||
            !authority(cookie, true) ||
            !selected_range_detail::read(current.Get(), cookie, actual, reads) || actual != source_ ||
            !authority(cookie, true)) return ApplyResult::rejected;
        // Explicit gravity keeps this private verification range around the
        // replacement. No numeric ACP/Word offsets are compared or assumed.
        if (FAILED(current->SetGravity(cookie, TF_GRAVITY_BACKWARD, TF_GRAVITY_FORWARD))) return ApplyResult::rejected;
        ComPtr<ITfRange> last;
        if (!selected_range_detail::selection(context_.Get(), cookie, last) ||
            !selected_range_detail::equal(last.Get(), current.Get(), context_.Get(), cookie) ||
            !authority(cookie, true)) return ApplyResult::rejected;
        ++mutations;
        if (FAILED(current->SetText(cookie, TF_ST_CORRECTION, replacement.data(), static_cast<LONG>(replacement.size()))) ||
            !selected_range_detail::read(current.Get(), cookie, actual, reads) || actual != replacement)
            return ApplyResult::mutation_unconfirmed;
        return ApplyResult::applied;
        } catch (...) {
            if (grant_) grant_->valid = false;
            return mutations != before ? ApplyResult::mutation_unconfirmed : ApplyResult::rejected;
        }
    }
};
}

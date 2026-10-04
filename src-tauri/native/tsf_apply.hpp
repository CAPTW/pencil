#pragma once
// Target-thread component. Not yet registered or reachable from the desktop UI.
// An adapter must bind the real context, revision sink, field eligibility and
// explicit user action before issuing a single-use grant. No cross-process
// verify/replace messages, clipboard, focus changes, retries or rollback.
#include <windows.h>
#include <msctf.h>
#include <wrl/client.h>
#include <atomic>
#include <chrono>
#include <memory>
#include <string>
#include <vector>
#include <functional>
#include <new>

namespace grammar {
using Microsoft::WRL::ComPtr;
enum class ApplyResult { pending, rejected, applied, mutation_unconfirmed };
struct ApplyGrant {
    std::atomic<bool> valid{true};
    std::atomic<unsigned long long> revision{0};
    bool field_eligible = false;
    std::chrono::steady_clock::time_point deadline;
    // Recheck target-side authority inside the lock, including before mutation.
    // Missing binding is never permission to use a caller-supplied context.
    std::function<bool(ITfContext*, TfEditCookie)> verify_binding;
};

class TsfApplySession final : public ITfEditSession {
    LONG refs_ = 1;
    ComPtr<ITfContext> context_;
    const std::wstring source_, replacement_;
    const LONG start_, end_;
    const unsigned long long revision_;
    std::shared_ptr<ApplyGrant> grant_;
    bool boundary(LONG offset) const {
        if (offset < 0 || static_cast<size_t>(offset) > source_.size()) return false;
        return offset == 0 || static_cast<size_t>(offset) == source_.size() ||
            !(source_[offset - 1] >= 0xd800 && source_[offset - 1] <= 0xdbff &&
              source_[offset] >= 0xdc00 && source_[offset] <= 0xdfff);
    }
    HRESULT read_all(TfEditCookie cookie, std::wstring& text) {
        ComPtr<ITfRange> range, end;
        HRESULT hr = context_->GetStart(cookie, &range); if (FAILED(hr)) return hr;
        hr = context_->GetEnd(cookie, &end); if (FAILED(hr)) return hr;
        hr = range->ShiftEndToRange(cookie, end.Get(), TF_ANCHOR_END); if (FAILED(hr)) return hr;
        // One extra unit detects overflow, rather than accepting a prefix.
        std::vector<WCHAR> buffer(65535); ULONG count = 0;
        ++text_reads;
        hr = range->GetText(cookie, 0, buffer.data(), static_cast<ULONG>(buffer.size()), &count);
        if (FAILED(hr) || count >= buffer.size()) return E_FAIL;
        text.assign(buffer.data(), count); return S_OK;
    }
public:
    ApplyResult result = ApplyResult::pending;
    bool called = false;
    unsigned mutation_calls = 0;
    unsigned text_reads = 0;
    unsigned diagnostic_stage = 0; // Content-free native qualification only.
    HRESULT diagnostic_read_result = E_PENDING;
    size_t diagnostic_text_units = 0;
    TsfApplySession(ITfContext* context, std::wstring source, LONG start, LONG end,
                   std::wstring replacement, unsigned long long revision,
                   std::shared_ptr<ApplyGrant> grant)
        : context_(context), source_(std::move(source)), replacement_(std::move(replacement)),
          start_(start), end_(end), revision_(revision), grant_(std::move(grant)) {}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID id, void** out) override {
        if (!out) return E_POINTER;
        *out = nullptr;
        if (id == IID_IUnknown || id == IID_ITfEditSession) { *out = static_cast<ITfEditSession*>(this); AddRef(); return S_OK; }
        return E_NOINTERFACE;
    }
    ULONG STDMETHODCALLTYPE AddRef() override { return InterlockedIncrement(&refs_); }
    ULONG STDMETHODCALLTYPE Release() override { auto n = InterlockedDecrement(&refs_); if (!n) delete this; return n; }
    HRESULT STDMETHODCALLTYPE DoEditSession(TfEditCookie cookie) override try {
        if (called) return E_UNEXPECTED;
        called = true; result = ApplyResult::rejected;
        diagnostic_stage = 1;
        if (!grant_ || !grant_->valid.exchange(false) || !grant_->field_eligible ||
            grant_->revision.load() != revision_ || std::chrono::steady_clock::now() >= grant_->deadline ||
            !context_ || source_.size() > 65534 || replacement_.empty() || replacement_.size() > 65534 ||
            source_.find(L'\0') != std::wstring::npos || replacement_.find(L'\0') != std::wstring::npos ||
            !grant_->verify_binding || !grant_->verify_binding(context_.Get(), cookie) ||
            start_ >= end_ || !boundary(start_) || !boundary(end_) ||
            source_.size() - (end_ - start_) + replacement_.size() > 65534) return S_FALSE;
        TF_STATUS status{};
        diagnostic_stage = 2;
        if (FAILED(context_->GetStatus(&status)) || (status.dwDynamicFlags & TF_SD_READONLY)) return S_FALSE;
        ComPtr<ITfContextComposition> compositions;
        diagnostic_stage = 3;
        ComPtr<IEnumITfCompositionView> enumeration;
        if (FAILED(context_.As(&compositions)) || FAILED(compositions->EnumCompositions(&enumeration))) return S_FALSE;
        ComPtr<ITfCompositionView> composing; ULONG count = 0;
        if (FAILED(enumeration->Next(1, &composing, &count)) || count) return S_FALSE;
        std::wstring actual;
        diagnostic_stage = 4;
        diagnostic_read_result = read_all(cookie, actual);
        diagnostic_text_units = actual.size();
        if (FAILED(diagnostic_read_result) || actual != source_) return S_FALSE;
        TF_SELECTION selection{}; ULONG fetched = 0;
        diagnostic_stage = 5;
        if (FAILED(context_->GetSelection(cookie, TF_DEFAULT_SELECTION, 1, &selection, &fetched)) || fetched != 1) return S_FALSE;
        ComPtr<ITfRange> range; range.Attach(selection.range);
        ComPtr<ITfRangeACP> acp;
        diagnostic_stage = 6;
        LONG start = -1, length = -1;
        if (FAILED(range.As(&acp)) || FAILED(acp->GetExtent(&start, &length)) ||
            start != start_ || length != end_ - start_) return S_FALSE;
        if (grant_->revision.load() != revision_ || !grant_->field_eligible ||
            std::chrono::steady_clock::now() >= grant_->deadline ||
            !grant_->verify_binding(context_.Get(), cookie)) return S_FALSE;
        diagnostic_stage = 7;
        // Everything above runs inside the same TSF write session as SetText.
        ++mutation_calls;
        result = ApplyResult::mutation_unconfirmed;
        HRESULT hr = range->SetText(cookie, 0, replacement_.data(), static_cast<LONG>(replacement_.size()));
        if (FAILED(hr)) return hr; // Never restore the whole document or replay.
        std::wstring expected = source_.substr(0, start_) + replacement_ + source_.substr(end_);
        if (FAILED(read_all(cookie, actual)) || actual != expected) return E_FAIL;
        result = ApplyResult::applied;
        return S_OK;
    } catch (const std::bad_alloc&) { return E_OUTOFMEMORY; }
      catch (...) { return E_FAIL; } // Preserve mutation_unconfirmed after SetText.
};
}

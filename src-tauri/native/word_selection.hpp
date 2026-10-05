#pragma once
// Inactive Word adapter component: selection metadata, NEVER text or mutation.
// The future target-side adapter must acquire the Application on its owning STA,
// authenticate the process/window, and bind an actual TSF revision/lease. These
// prerequisites are not inferred from an IDispatch pointer or this result.
#include "word_privacy.hpp"
#include <wrl/client.h>
#include <atomic>
#include <cstdint>
#include <memory>

namespace grammar {
struct WordObservationGuard {
    const DWORD thread;
    const ULONGLONG deadline;
    const uint64_t revision;
    const std::atomic<uint64_t>& current_revision;
    const std::atomic<bool>& enabled;
    bool current() const {
        return GetCurrentThreadId()==thread && GetTickCount64()<deadline &&
            enabled.load() && current_revision.load()==revision;
    }
};
namespace word_selection_detail {
using Microsoft::WRL::ComPtr;
inline bool same(IUnknown* left,IUnknown* right) {
    if(!left || !right)return false;
    ComPtr<IUnknown> a,b;
    return left->QueryInterface(IID_PPV_ARGS(&a))==S_OK &&
        right->QueryInterface(IID_PPV_ARGS(&b))==S_OK && a.Get()==b.Get();
}
inline bool object(IDispatch* from,const wchar_t* name,ComPtr<IDispatch>& result,const WordObservationGuard& guard) {
    if(!guard.current())return false;
    word_privacy_detail::Value value;
    if(!word_privacy_detail::get(from,name,value) || !guard.current() ||
        value.value.vt!=VT_DISPATCH || !value.value.pdispVal)return false;
    result=value.value.pdispVal;return guard.current();
}
inline bool number(IDispatch* from,const wchar_t* name,LONG& result,const WordObservationGuard& guard) {
    if(!guard.current())return false;
    word_privacy_detail::Value value;
    if(!word_privacy_detail::get(from,name,value) || !guard.current() || value.value.vt!=VT_I4)return false;
    result=value.value.lVal;return true;
}
struct Metadata {
    ComPtr<IDispatch> document,window,selection;
    LONG start=0,end=0;
};
inline bool observe(IDispatch* app,HWND expected,Metadata& result,const WordObservationGuard& guard) {
    if(!app || !expected || !guard.current())return false;
    ComPtr<IDispatch> document,window,windowDocument,selection,selectionDocument,range,rangeDocument,font;
    LONG hwnd=0,kind=0,story=0,start=0,end=0,hidden=0;
    if(!object(app,L"ActiveWindow",window,guard) || !number(window.Get(),L"Hwnd",hwnd,guard) ||
        hwnd!=static_cast<LONG>(reinterpret_cast<INT_PTR>(expected)) ||
        !object(window.Get(),L"Document",document,guard) ||
        !object(app,L"ActiveDocument",windowDocument,guard) || !same(document.Get(),windowDocument.Get()))return false;
    if(word_privacy_metadata(document.Get(),guard.thread,guard.current())!=WordPrivacyMetadata::clear || !guard.current())return false;
    if(!object(window.Get(),L"Selection",selection,guard) ||
        !object(selection.Get(),L"Document",selectionDocument,guard) || !same(document.Get(),selectionDocument.Get()) ||
        !number(selection.Get(),L"Type",kind,guard) || kind!=2 || // wdSelectionNormal
        !object(selection.Get(),L"Range",range,guard) ||
        !object(range.Get(),L"Document",rangeDocument,guard) || !same(document.Get(),rangeDocument.Get()) ||
        !number(range.Get(),L"StoryType",story,guard) || story!=1 || // main story only initially
        !number(range.Get(),L"Start",start,guard) || !number(range.Get(),L"End",end,guard) ||
        start<0 || end<=start || static_cast<int64_t>(end)-start>8192 ||
        !object(range.Get(),L"Font",font,guard) || !number(font.Get(),L"Hidden",hidden,guard) || hidden!=0)return false;
    // Complex/structural selections need separate qualification. Unknown count
    // or mixed hidden formatting is not treated as plain visible text.
    for(const auto* name:{L"Fields",L"ContentControls",L"Tables",L"InlineShapes"}) {
        ComPtr<IDispatch> collection;LONG count=-1;
        if(!object(range.Get(),name,collection,guard) || !number(collection.Get(),L"Count",count,guard) || count!=0)return false;
    }
    if(word_privacy_metadata(document.Get(),guard.thread,guard.current())!=WordPrivacyMetadata::clear || !guard.current())return false;
    result={document,window,selection,start,end};return true;
}
inline bool matches(const Metadata& a,const Metadata& b) {
    return same(a.document.Get(),b.document.Get()) && same(a.window.Get(),b.window.Get()) &&
        same(a.selection.Get(),b.selection.Get()) && a.start==b.start && a.end==b.end;
}
}
// Keep and destroy on the target STA. No interface/text pointer is exposed to
// callers; a match is metadata consistency, NOT permission to read/write text.
class WordSelectionIdentity final {
    friend class WordCorrelationLease;
    word_selection_detail::Metadata metadata_;
    Microsoft::WRL::ComPtr<IDispatch> app_;
    HWND window_;
    const DWORD thread_;
    const uint64_t revision_;
    WordSelectionIdentity(IDispatch* app,HWND window,const WordObservationGuard& guard,
                          word_selection_detail::Metadata metadata)
        :metadata_(std::move(metadata)),app_(app),window_(window),thread_(guard.thread),revision_(guard.revision){}
public:
    WordSelectionIdentity(const WordSelectionIdentity&)=delete;
    WordSelectionIdentity& operator=(const WordSelectionIdentity&)=delete;
    static std::unique_ptr<WordSelectionIdentity> capture(IDispatch* app,HWND window,const WordObservationGuard& guard) {
        word_selection_detail::Metadata first,second;
        if(!word_selection_detail::observe(app,window,first,guard) ||
            !word_selection_detail::observe(app,window,second,guard) ||
            !word_selection_detail::matches(first,second) || !guard.current())return {};
        return std::unique_ptr<WordSelectionIdentity>(new WordSelectionIdentity(app,window,guard,std::move(first)));
    }
    bool matches(const WordObservationGuard& guard) const {
        if(guard.thread!=thread_ || guard.revision!=revision_ || !guard.current())return false;
        word_selection_detail::Metadata current;
        return word_selection_detail::observe(app_.Get(),window_,current,guard) &&
            word_selection_detail::matches(metadata_,current) && guard.current();
    }
};
}

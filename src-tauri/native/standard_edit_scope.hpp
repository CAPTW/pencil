#pragma once
#include "tsf_capture.hpp"
#include <array>
#ifdef GRAMMAR_NATIVE_TEST_DIAGNOSTICS
#include <cstdio>
#endif

namespace grammar {
struct StandardScopeIdentity {
    std::array<InputScope,32> values{};
    UINT size=0;
    bool operator==(const StandardScopeIdentity& other)const {
        if(!size||size!=other.size)return false;
        for(UINT i=0;i<size;++i)if(values[i]!=other.values[i])return false;
        return true;
    }
};
// Metadata observer: no view/focus/editability admission and no text reads.
// The mutation path separately validates the live context and field below.
inline bool standard_edit_scope_value(ITfContext* context,TfEditCookie cookie,StandardScopeIdentity* identity=nullptr,bool* absent=nullptr) {
    if(identity)*identity=StandardScopeIdentity{};
    if(absent)*absent=false;
    if(!context)return false;
    ComPtr<ITfReadOnlyProperty> property; ComPtr<ITfRange> range, end;
    if (context->GetAppProperty(input_scope_property, &property) != S_OK || !property ||
        FAILED(context->GetStart(cookie, &range)) || !range || FAILED(context->GetEnd(cookie, &end)) || !end ||
        FAILED(range->ShiftEndToRange(cookie, end.Get(), TF_ANCHOR_END))) return false;
    VARIANT value; VariantInit(&value);
    HRESULT hr = property->GetValue(cookie, range.Get(), &value);
    if(absent)*absent=SUCCEEDED(hr)&&value.vt==VT_EMPTY;
    ComPtr<ITfInputScope> scope; bool allowed = false;
    if (SUCCEEDED(hr) && value.vt == VT_UNKNOWN && value.punkVal &&
        SUCCEEDED(value.punkVal->QueryInterface(IID_PPV_ARGS(&scope)))) {
        InputScope* scopes = nullptr; UINT size = 0;
        hr = scope->GetInputScopes(&scopes, &size);
        allowed = SUCCEEDED(hr) && scopes && size > 0 && size <= 32;
        if (allowed) for (UINT i = 0; i < size; ++i)
            if (scopes[i] != IS_DEFAULT && scopes[i] != IS_TEXT) allowed = false;
        if(allowed&&identity){identity->size=size;for(UINT i=0;i<size;++i)identity->values[i]=scopes[i];}
        CoTaskMemFree(scopes);
    }
#ifdef GRAMMAR_NATIVE_TEST_DIAGNOSTICS
    printf("standard_scope_property vt=%u allowed=%d\n",static_cast<unsigned>(value.vt),allowed);fflush(stdout);
#endif
    VariantClear(&value);
    return allowed;
}
// Metadata only. Never infer a plain-text scope from absence or a filename.
inline bool standard_edit_plain_scope(ITfContext* context,TfEditCookie cookie,HWND field,StandardScopeIdentity* identity=nullptr) {
    if(identity)*identity=StandardScopeIdentity{};
    if (!context) return false;
    ComPtr<ITfContextView> view; HWND actual = nullptr;
    ComPtr<ITfDocumentMgr> document; ComPtr<ITfContext> top;
    TF_STATUS status{};
    if (FAILED(context->GetActiveView(&view)) || !view || FAILED(view->GetWnd(&actual)) || actual != field ||
        FAILED(context->GetDocumentMgr(&document)) || !document || FAILED(document->GetTop(&top)) ||
        !same_com(context, top.Get()) || FAILED(context->GetStatus(&status)) || (status.dwDynamicFlags & TF_SD_READONLY)) return false;
    ComPtr<ITfContextComposition> composition; ComPtr<IEnumITfCompositionView> enumeration;
    ComPtr<ITfCompositionView> item; ULONG count = 0;
    if (FAILED(context->QueryInterface(IID_PPV_ARGS(&composition))) ||
        FAILED(composition->EnumCompositions(&enumeration)) || !enumeration ||
        FAILED(enumeration->Next(1, &item, &count)) || count) return false;
    return standard_edit_scope_value(context,cookie,identity);
}
}

#pragma once
// Word adapter prerequisite, not capture/mutation authority. NOT ACTIVATED.
// Only use an already authenticated target-thread document. No ROT lookup,
// application activation, text/file/password reads, setters or event disabling.
#include <windows.h>
#include <oaidl.h>
#include <initializer_list>

namespace grammar {
enum class WordPrivacyMetadata { unknown, denied, clear };
namespace word_privacy_detail {
struct Value final {
    VARIANT value;
    Value(){VariantInit(&value);}
    ~Value(){VariantClear(&value);}
    Value(const Value&)=delete;
    Value& operator=(const Value&)=delete;
};
inline bool get(IDispatch* object,const wchar_t* property,Value& value) {
    if(!object)return false;
    LPOLESTR name=const_cast<LPOLESTR>(property);DISPID id=DISPID_UNKNOWN;
    if(object->GetIDsOfNames(IID_NULL,&name,1,LOCALE_USER_DEFAULT,&id)!=S_OK)return false;
    DISPPARAMS parameters{};
    EXCEPINFO exception{};UINT bad=0;
    const auto hr=object->Invoke(id,IID_NULL,LOCALE_USER_DEFAULT,DISPATCH_PROPERTYGET,
        &parameters,&value.value,&exception,&bad);
    // Do not invoke deferred exception callbacks or emit possibly sensitive
    // exception descriptions. Release any COM-owned diagnostic strings.
    SysFreeString(exception.bstrSource);SysFreeString(exception.bstrDescription);
    SysFreeString(exception.bstrHelpFile);
    return hr==S_OK;
}
inline WordPrivacyMetadata false_flag(IDispatch* object,const wchar_t* name) {
    Value result;
    if(!get(object,name,result) || result.value.vt!=VT_BOOL ||
        (result.value.boolVal!=VARIANT_FALSE && result.value.boolVal!=VARIANT_TRUE))return WordPrivacyMetadata::unknown;
    return result.value.boolVal==VARIANT_FALSE?WordPrivacyMetadata::clear:WordPrivacyMetadata::denied;
}
}
inline WordPrivacyMetadata word_privacy_metadata(IDispatch* document,DWORD ownerThread,bool optIn) {
    // An explicit request permits metadata inspection, never product activation.
    if(!optIn || !document || GetCurrentThreadId()!=ownerThread)return WordPrivacyMetadata::unknown;
    using namespace word_privacy_detail;
    // HasPassword/WriteReserved are Boolean metadata. Never use Password,
    // WritePassword, FullName, Content, Text or credential-bearing properties.
    for(const auto* property:{L"HasPassword",L"WriteReserved",L"ReadOnly"}) {
        const auto status=false_flag(document,property);
        if(status!=WordPrivacyMetadata::clear)return status;
    }
    Value protection;
    if(!get(document,L"ProtectionType",protection) || protection.value.vt!=VT_I4)return WordPrivacyMetadata::unknown;
    if(protection.value.lVal!=-1)return WordPrivacyMetadata::denied; // wdNoProtection
    Value permission;
    if(!get(document,L"Permission",permission) || permission.value.vt!=VT_DISPATCH || !permission.value.pdispVal)return WordPrivacyMetadata::unknown;
    return false_flag(permission.value.pdispVal,L"Enabled");
}
// A clear result is NOT a document identity, range, revision or sensitivity
// guarantee. Field-level eligibility, TSF lock/identity binding, revalidation,
// cancellation, deployment and actual Word qualification are still required.
}

#include "word-metadata-fixture.hpp"
int wmain(){
    unsigned checks=0;auto require=[&](bool value){if(!value){printf("FAIL %u\n",checks+1);return false;}++checks;return true;};
    {
        Fixture f;auto identity=f.capture();if(!require(identity && identity->matches(f.guard())))return 1;
        f.selection.object(L"Document",&f.alias);if(!require(identity->matches(f.guard())))return 1;
        f.range.number(L"Start",5);if(!require(!identity->matches(f.guard())))return 1;
        identity.reset();if(!require(f.balanced()))return 1;
    }
    for(unsigned scenario=0;scenario<26;++scenario){
        Fixture f;
        switch(scenario){
        case 0:f.enabled=false;break;
        case 1:f.revision=8;break;
        case 2:f.hwnd=nullptr;break;
        case 3:f.window.number(L"Hwnd",4321);break;
        case 4:f.app.object(L"ActiveDocument",&f.other);break;
        case 5:f.selection.object(L"Document",&f.other);break;
        case 6:f.range.object(L"Document",&f.other);break;
        case 7:f.selection.number(L"Type",1);break;
        case 8:f.selection.number(L"Type",6);break;
        case 9:f.range.number(L"StoryType",2);break;
        case 10:f.range.number(L"Start",-1);break;
        case 11:f.range.number(L"End",4);break;
        case 12:f.range.number(L"End",3);break;
        case 13:f.range.number(L"End",INT32_MAX);break;
        case 14:f.font.number(L"Hidden",-1);break;
        case 15:f.font.number(L"Hidden",9999999);break;
        case 16:f.fields.number(L"Count",1);break;
        case 17:f.controls.number(L"Count",1);break;
        case 18:f.tables.number(L"Count",1);break;
        case 19:f.shapes.number(L"Count",1);break;
        case 20:f.document.flag(L"HasPassword",true);break;
        case 21:f.document.flag(L"WriteReserved",true);break;
        case 22:f.document.flag(L"ReadOnly",true);break;
        case 23:f.document.number(L"ProtectionType",3);break;
        case 24:f.permission.flag(L"Enabled",true);break;
        case 25:f.window.object(L"Selection",nullptr);break;
        }
        if(!require(!f.capture() && f.balanced()))return 1;
        if(scenario>=20 && scenario<=24 && !require(f.selection.invokes==0 && f.range.invokes==0))return 1;
    }
    for(unsigned fault=0;fault<3;++fault){
        Fixture f;
        for(auto pair: {std::make_pair(&f.app,L"ActiveWindow"),{&f.window,L"Hwnd"},{&f.range,L"Start"},{&f.font,L"Hidden"},{&f.fields,L"Count"}}){
            auto* property=fault==0?&pair.first->missing:fault==1?&pair.first->wrong:&pair.first->failed;
            *property=pair.second;if(!require(!f.capture() && f.balanced()))return 1;property->clear();
        }
    }
    {
        Fixture f;const auto expired=WordObservationGuard{f.thread,GetTickCount64(),7,f.revision,f.enabled};
        if(!require(!WordSelectionIdentity::capture(&f.app,f.hwnd,expired) && !f.app.invokes))return 1;
        bool refused=false;std::thread worker([&]{refused=!f.capture();});worker.join();
        if(!require(refused && !f.app.invokes && f.balanced()))return 1;
    }
    for(unsigned scenario=0;scenario<4;++scenario){
        Fixture f;unsigned observed=0;
        f.range.onInvoke=[&](const std::wstring& name){if(name!=L"End")return;
            if(scenario==0){++f.revision;} // Revision changes even if range/source restored.
            if(scenario==1){f.enabled=false;}
            if(scenario==2 && ++observed==2){f.range.number(L"End",17);}
            if(scenario==3){f.document.flag(L"ReadOnly",true);}
        };
        if(!require(!f.capture() && f.balanced()))return 1;
    }
    {
        Fixture f;auto identity=f.capture();if(!require(static_cast<bool>(identity)))return 1;
        ++f.revision;auto newer=WordObservationGuard{f.thread,GetTickCount64()+5000,8,f.revision,f.enabled};
        if(!require(!identity->matches(newer)))return 1;
        identity.reset();if(!require(f.balanced()))return 1;
    }
    printf("PASS %u inert Word selection checks; text reads 0; Office NOT_RUN\n",checks);return 0;
}

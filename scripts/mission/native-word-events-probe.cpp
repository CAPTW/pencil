#include "word-events-fixture.hpp"
struct Port final:WordNativeWindowPort {
    Fixture* f;explicit Port(Fixture* value):f(value){}
    bool eligible(HWND field,HWND root,DWORD thread) override{return field==reinterpret_cast<HWND>(4321) && root==f->hwnd && thread==GetCurrentThreadId();}
    HRESULT native_window(HWND,IDispatch** out) override{*out=&f->window;f->window.AddRef();return S_OK;}
};
int wmain() {
    unsigned checks=0;auto require=[&](bool ok){++checks;if(!ok)printf("FAIL %u\n",checks);return ok;};
    for(unsigned scenario=0;scenario<9;++scenario) {
        Fixture f;Events events;f.app.extension=static_cast<IConnectionPointContainer*>(&events);f.window.object(L"Application",&f.app);
        auto state=std::make_shared<WordTsfState>();state->enabled=true;state->revision=7;
        auto port=std::make_shared<Port>(&f);
        WordObservationGuard guard{state->thread,GetTickCount64()+5000,7,state->revision,state->enabled};
        auto target=WordTargetIdentity::capture(port,reinterpret_cast<HWND>(4321),f.hwnd,guard);
        if(!require(static_cast<bool>(target)))return 1;
        if(scenario==1)events.missing=true;
        if(scenario==2)events.wrongInterface=true;
        if(scenario==3)events.adviseResult=CONNECT_E_ADVISELIMIT;
        if(scenario==4)events.nativeCookie=0xffffffff;
        if(scenario==5)events.onAdvise=[&]{events.fire();};
        if(scenario==6)events.nativeCookie=0;
        auto lifetime=std::make_shared<int>(0);std::weak_ptr<void> retained=lifetime;
        {
            WordEventBinding binding;
            const bool attached=binding.attach(state,*target,guard,lifetime);lifetime.reset();
            const bool expected=scenario==0 || scenario==4 || scenario==7 || scenario==8;
            if(!require(attached==expected))return 1;
            if(expected) {
                if(!require(binding.matches(&f.app) && !binding.matches(&f.other)))return 1;
                auto grant=std::make_shared<SelectedRangeGrant>();state->outstanding=grant;
                if(!require(events.fire(99999)==S_OK && state->revision==8 && !grant->valid))return 1;
                // Poison argument pointers must not be inspected or modified.
                if(!require(events.sink->Invoke(12,IID_NULL,0,DISPATCH_METHOD,reinterpret_cast<DISPPARAMS*>(1),reinterpret_cast<VARIANT*>(1),reinterpret_cast<EXCEPINFO*>(1),reinterpret_cast<UINT*>(1))==S_OK))return 1;
                HRESULT hr=S_OK;std::thread other([&]{hr=events.fire();});other.join();
                if(!require(hr==RPC_E_WRONG_THREAD && !state->enabled))return 1;
            }
            if(scenario==7) {
                events.closeResult=E_FAIL;
                if(!require(!binding.close() && !retained.expired() && !binding.matches(&f.app)))return 1;
                events.closeResult=S_OK;
            }
            if(scenario==8) {
                state.reset();if(!require(events.fire()==S_OK))return 1;
            }
            if(scenario==6) {
                if(!require(!binding.close() && !retained.expired() && events.unadvises==0))return 1;
            } else if(!require(binding.close() && retained.expired()))return 1;
        }
        events.sink.Reset();target.reset();
        if(!require(retained.expired() && events.refs==1 && f.balanced() && f.range.textReads==0))return 1;
    }
    printf("PASS %u Word event subscription/invalidation checks; Office NOT_RUN; event args unread\n",checks);return 0;
}

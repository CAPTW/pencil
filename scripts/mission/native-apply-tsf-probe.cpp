// Owned Windows Sandbox feasibility probe, not a production adapter.
// Compile with the installed Windows SDK; no registration, injection or hooks.
#include <windows.h>
#include <msctf.h>
#include <richedit.h>
#include <wrl/client.h>
#include <cstdio>
#include <string>
using Microsoft::WRL::ComPtr;

static void pump(DWORD milliseconds) {
    auto end = GetTickCount64() + milliseconds;
    do {
        MSG msg;
        while (PeekMessageW(&msg, nullptr, 0, 0, PM_REMOVE)) {
            TranslateMessage(&msg); DispatchMessageW(&msg);
        }
        Sleep(5);
    } while (GetTickCount64() < end);
}

#include "../../src-tauri/native/tsf_apply.hpp"

int wmain(int argc, wchar_t** argv) {
    const bool standardEdit = argc == 2 && std::wstring(argv[1]) == L"--standard-edit";
    WCHAR user[128]{}; GetEnvironmentVariableW(L"USERNAME", user, 128);
    if (std::wstring(user) != L"WDAGUtilityAccount") { puts("REFUSED: Sandbox only"); return 2; }
    HRESULT hr = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    if (FAILED(hr)) return 3;
    int result = 1;
    {
        ComPtr<ITfThreadMgr> manager;
        hr = CoCreateInstance(CLSID_TF_ThreadMgr, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(&manager));
        TfClientId client = 0;
        if (FAILED(hr) || FAILED(hr = manager->Activate(&client))) { printf("activate=%08lx\n", hr); return 4; }
        LoadLibraryW(L"Msftedit.dll");
        HWND top = CreateWindowExW(0, L"STATIC", L"Grammar owned TSF Apply probe", WS_OVERLAPPEDWINDOW | WS_VISIBLE, 30, 30, 600, 250, nullptr, nullptr, GetModuleHandleW(nullptr), nullptr);
        HWND edit = CreateWindowExW(0, standardEdit ? L"Edit" : L"RICHEDIT50W", L"hello world", WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_MULTILINE, 10, 10, 550, 150, top, nullptr, GetModuleHandleW(nullptr), nullptr);
        if (!top || !edit) { puts("window_creation_failed"); return 5; }
        if (!standardEdit) SendMessageW(edit, EM_SETEDITSTYLE, SES_USECTF, SES_USECTF);
        SetForegroundWindow(top); SetFocus(edit); pump(500);
        ComPtr<ITfDocumentMgr> document;
        hr = manager->GetFocus(&document);
        ComPtr<ITfContext> context;
        if (SUCCEEDED(hr) && document) hr = document->GetTop(&context);
        if (FAILED(hr) || !context) {
            printf("NOT_SUPPORTED: no owned TSF context standard_edit=%d hr=%08lx\n", standardEdit, hr);
        } else {
            result = 0;
            for (int scenario = 0; scenario < 13; ++scenario) {
                const auto source = scenario == 11 ? L"world world" : L"hello world";
                SetWindowTextW(edit, scenario == 1 ? L"hello revised" : source);
                SendMessageW(edit, EM_SETSEL, scenario == 2 || scenario == 11 ? 0 : 6, scenario == 2 || scenario == 11 ? 5 : 11);
                pump(50);
                auto grant = std::make_shared<grammar::ApplyGrant>();
                // Legacy transaction-only probe. Real capture/binding is tested
                // separately; this is deliberately synthetic authority.
                grant->verify_binding = [expected = context.Get()](ITfContext* current, TfEditCookie) { return current == expected; };
                // Binding checks can reenter the target. Recheck authority after
                // that callback, not only before it.
                unsigned checks = 0;
                if (scenario == 12) grant->verify_binding = [&, expected = context.Get()](ITfContext* current, TfEditCookie) {
                    if (++checks == 2) ++grant->revision;
                    return current == expected;
                };
                grant->field_eligible = scenario != 5;
                grant->deadline = std::chrono::steady_clock::now() + std::chrono::seconds(scenario == 4 ? -1 : 5);
                if (scenario == 3) grant->valid = false;
                if (scenario == 6) grant->revision = 1;
                auto session = new grammar::TsfApplySession(context.Get(), source, 6, 11, L"Grammar", 0, grant);
                HRESULT sessionResult = E_PENDING;
                hr = context->RequestEditSession(client, session, TF_ES_ASYNC | TF_ES_READWRITE, &sessionResult);
                bool queued = !session->called && sessionResult == TF_S_ASYNC;
                // Reproduce changes after request admission, before the write
                // lock is granted. The callback must revalidate, not trust IPC.
                if (scenario == 7) SetWindowTextW(edit, L"hello changedlater");
                if (scenario == 8) SendMessageW(edit, EM_SETSEL, 0, 5);
                if (scenario == 9) grant->valid = false;
                if (scenario == 10) ++grant->revision;
                for (int i = 0; i < 500 && !session->called && SUCCEEDED(hr); ++i) pump(10);
                WCHAR actual[128]{}; GetWindowTextW(edit, actual, 128);
                const auto expected = scenario == 0 ? L"hello Grammar" : scenario == 1 ? L"hello revised" : scenario == 7 ? L"hello changedlater" : source;
                bool pass = queued && session->called && std::wstring(actual) == expected && (session->result == grammar::ApplyResult::applied) == (scenario == 0) && session->mutation_calls == (scenario == 0 ? 1u : 0u);
                printf("case=%d pass=%d callback=%d changed=%d request=%08lx session=%08lx reread_match=%d\n", scenario, pass, session->called, session->mutation_calls, hr, sessionResult, std::wstring(actual) == expected);
                if (!pass) result = 1;
                printf("stage=%u read=%08lx units=%zu\n", session->diagnostic_stage, session->diagnostic_read_result, session->diagnostic_text_units);
                session->Release();
                if (scenario == 0) {
                    const auto canUndo = SendMessageW(edit, EM_CANUNDO, 0, 0);
                    const auto undo = SendMessageW(edit, EM_UNDO, 0, 0); pump(100);
                    GetWindowTextW(edit, actual, 128);
                    const bool restored = canUndo && undo && std::wstring(actual) == source;
                    printf("undo pass=%d available=%lld result=%lld\n", restored,
                        static_cast<long long>(canUndo), static_cast<long long>(undo));
                    if (!restored) result = 1;

                    // Restore the fixture externally, then replay the consumed
                    // grant. Matching content must not revive old authority.
                    SetWindowTextW(edit, source); SendMessageW(edit, EM_SETSEL, 6, 11); pump(50);
                    auto replay = new grammar::TsfApplySession(context.Get(), source, 6, 11, L"Grammar", 0, grant);
                    hr = context->RequestEditSession(client, replay, TF_ES_ASYNC | TF_ES_READWRITE, &sessionResult);
                    for (int i = 0; i < 500 && !replay->called && SUCCEEDED(hr); ++i) pump(10);
                    GetWindowTextW(edit, actual, 128);
                    bool rejected = replay->called && replay->mutation_calls == 0 && replay->result == grammar::ApplyResult::rejected && std::wstring(actual) == source;
                    printf("replay pass=%d\n", rejected); if (!rejected) result = 1;
                    replay->Release();
                }
            }
        }
        DestroyWindow(top);
        manager->Deactivate();
    }
    CoUninitialize();
    return result;
}

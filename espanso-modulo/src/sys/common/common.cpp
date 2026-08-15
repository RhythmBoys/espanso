/*
 * This file is part of modulo.
 *
 * Copyright (C) 2020-2021 Federico Terzi
 *
 * modulo is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * modulo is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with modulo.  If not, see <https://www.gnu.org/licenses/>.
 */

#include "common.h"

#include <wx/fontenum.h>

#ifdef __WXMSW__
#include <windows.h>
#endif
#ifdef __WXOSX__
#include "mac.h"
#endif

void setFrameIcon(wxString iconPath, wxFrame *frame) {
    if (!iconPath.IsEmpty()) {
        wxBitmapType imgType = wxICON_DEFAULT_TYPE;

#ifdef __WXMSW__
        imgType = wxBITMAP_TYPE_ICO;
#endif

        wxIcon icon;
        icon.LoadFile(iconPath, imgType);
        if (icon.IsOk()) {
            frame->SetIcon(icon);
        }
    }
}

void Activate(wxFrame *frame) {
#ifdef __WXMSW__

    HWND handle = frame->GetHandle();
    if (handle == GetForegroundWindow()) {
        return;
    }

    if (IsIconic(handle)) {
        ShowWindow(handle, 9);
    }

    INPUT ip;
    ip.type = INPUT_KEYBOARD;
    ip.ki.wScan = 0;
    ip.ki.time = 0;
    ip.ki.dwExtraInfo = 0;
    ip.ki.wVk = VK_MENU;
    ip.ki.dwFlags = 0;

    SendInput(1, &ip, sizeof(INPUT));
    ip.ki.dwFlags = KEYEVENTF_KEYUP;

    SendInput(1, &ip, sizeof(INPUT));

    SetForegroundWindow(handle);

#endif
#ifdef __WXOSX__
    ActivateApp();
#endif
}

void SetupWindowStyle(wxFrame *frame) {
#ifdef __WXOSX__
    SetWindowStyles((NSWindow *)frame->MacGetTopLevelWindowRef());
#endif
}

wxString PreferredUiFontFace() {
    // wxHtmlListBox only honours a single face name reliably; pick the first
    // installed family that typically carries CJK glyphs so Chinese is not □.
#ifdef __WXMSW__
    static const wxChar *const candidates[] = {
        wxT("Microsoft YaHei UI"),
        wxT("Microsoft YaHei"),
        wxT("微软雅黑"),
        wxT("SimSun"),
        wxT("宋体"),
        wxT("Segoe UI"),
    };
#elif defined(__WXOSX__)
    static const wxChar *const candidates[] = {
        wxT("PingFang SC"),
        wxT("Hiragino Sans GB"),
        wxT("Heiti SC"),
        wxT("Helvetica"),
    };
#else
    static const wxChar *const candidates[] = {
        wxT("Noto Sans CJK SC"),
        wxT("Noto Sans SC"),
        wxT("WenQuanYi Micro Hei"),
        wxT("Sans"),
    };
#endif
    for (const wxChar *name : candidates) {
        if (wxFontEnumerator::IsValidFacename(name)) {
            return wxString(name);
        }
    }
    return wxString(candidates[0]);
}


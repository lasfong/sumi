# WEBSOCKET TEST MATRIX - chaos

Frame chuan: {type, seq, current_index, payload}; types: state, candle, error, completed, playback_state echo.

| ID | Chaos | Pre | Hanh vi | Expected |
|---|---|---|---|---|
| WS-01 | Restart BE giua play | playing seq=50 | FE backoff reconnect, resume tu echo | Khong busy-loop, cursor khong lech, tiep tuc seq=51+ |
| WS-02 | Mat mang 10s (offline) | playing | online/offline banner, pause hien thi | Reconnect tu dong, echo index dung |
| WS-03 | JSON rac | connected | BE frame error, khong crash | FE toast, giu ket noi |
| WS-04 | Speed am/qua lon | connected | BE clamp + echo | FE hien speed thuc te tu echo |
| WS-05 | set_speed lien tuc | playing | echo moi lenh | Toc do cuoi thang, khong mat tick |
| WS-06 | Step nhanh 20 lan (REST+WS) | pause | token/huy, disable step pending | Index cuoi = server, khong duplicate candle |
| WS-07 | Switch session giua play | playing S1 | close WS cu, cancelAll, mo WS moi | Khong flash S1, khong nhan msg cu |
| WS-08 | Send khi closed | closed | check send return | FE queue hoac bao loi, khong silent-drop |
| WS-09 | completed cuoi data | index=N-1 | frame completed | FE pause + banner het du lieu |
| WS-10 | 2 tab cung session | 2 FE | moi tab seq doc lap | Khong cross-talk cursor |

Automation: Playwright + WS mock/intercept; log seq/index moi frame; assert no-busy-loop (reconnect interval tang dan).

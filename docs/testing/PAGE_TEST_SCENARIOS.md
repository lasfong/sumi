# PAGE TEST SCENARIOS - Sumi V3 (browser 1440x1000 + 1280x800, temp-DB, no console errors)

Format moi case: TestID | Page | Priority | Pre | Steps | ExpectedUI | ExpectedBE | Invariant | Negative | Automation | Evidence.
Pri: P0 = blocker, P1 = nen co. Tat ca temp-DB; hash backend/sumi.db unchanged.

## 0. App boot / nav (G/R)

- NAV-001 | App boot | P0 | BE+FE chay, temp-DB | Mo / | Sidebar 2 nhom, khong trang trang, khong console error | GET health 200 | G-01 |Mat mang -> error boundary co reset, khong trang trang| Playwright boot | screenshot + console log |
- NAV-002 | Sidebar | P0 | boot | Click tung item /replay /strategy-lab /scanner /journal /analytics /import | Active dung, routes dung | - | - | /backtest cu redirect /strategy-lab?tab=single, khong 404 cam | Playwright | screenshot |
- NAV-003 | Deep-link session | P0 | co session S1 | Mo /replay?session=S1 | Dung symbol/date/index, OHLCV hien khong can crosshair (R-02) | GET session/candles limit=current_index+1 | R-05 resume | session sai -> 404 UI + toast, khong crash | Playwright | screenshot |
- NAV-004 | Reload/resume | P0 | dang replay index N | Reload | Index N + workspace (indicators/drawings/panels) restore | cursor tu BE | R-05 | - | Playwright reload | screenshot before/after |
- NAV-005 | Min-width/mobile | P1 | - | Resize 1280x800; mo mobile emulation | 1280 dung duoc; mobile banner ro read-only/limited | - | viewport evidence | Khong gia vo full-trading tren mobile | Playwright viewport | screenshots 2 size |

## 1. Replay (R-01..R-05, T-04)

- REPLAY-001 | Replay session create | P0 | co candles SSI | Chon symbol+date -> Create | Vao workspace index 0, 1 nen | POST session 201, candles slice | R-01 | Trung session -> dung lai, khong duplicate | API+UI | id |
- REPLAY-002 | Step next/prev +-5 | P0 | session index N | Click next, prev, +5/-5, phim mui ten | Chart+positions+orders+markers+indicators update, khong duplicate/race | limit=N+1, practice-state filtered | R-03/R-04 | Step nhanh 20 lan van dung index cuoi (gap FE-01) | Playwright stress | video/shot |
- REPLAY-003 | Play/pause/speed | P0 | session | Play 2x -> pause; doi speed 1/2/5/10 | Cho chay deu, pause dung ngay | WS echo playback_state+speed; BE clamp | R-03 | Restart BE giua play (xem WS-01) | Playwright+WS | tick log |
- REPLAY-004 | Lookahead raw ledger | P0 | session N + decision tai N+3 (seed) | GET decisions/orders/trades | Khong thay activity >N; chi practice-state | 200 filtered candle_index<=N | R-01 | Goi raw truc tiep van filter | pytest+Playwright | payload dump |
- REPLAY-005 | Projected drop | P0 | Ichimoku tren SSI dai | Bat Ichimoku tai index N | Khong ve Senkou +26 tuong lai; warming N/X neu thieu | API khong tra rows >max_visible_ts | R-01/I-13 | - | pytest+shot | payload+shot |
- REPLAY-006 | Timeframe D/1W | P0 | session | Doi D<->1W | Series dung TF, khong tron cache; key du TF | MTF timestamp<= | R-04 | Prefix-cache cu sai TF (gap) | Playwright | shots 2 TF |
- REPLAY-007 | Indicators add/edit/hide/remove | P0 | session | Them SMA20+EMA50, RSI, MACD; edit params; hide/show; xoa 1 | List luon thay + phan biet type/params/pane/mau (I-01); multi-instance phan biet (I-06); validate tu registry (I-05) | compute BE, fail-closed | I-01..I-07 | Params sai -> loi ro, khong crash | Playwright | shots |
- REPLAY-008 | Panes + MACD/RSI/CCI refs | P1 | co indicators | Resize/reorder pane; check MACD line/signal/hist/zero (I-10), RSI 30/50/70 (I-11), CCI -100/0/100 (I-12) | Tieu de/legend/values/controls + chieu cao dung duoc (I-08) | - | I-08..I-12 | Warmup null khong ve doan ao (I-13) | Playwright | shots |
- REPLAY-009 | Drawings 7 tools | P0 | session | Ve Horizontal/Trendline/Ray/Rectangle/Fibo/Text + RiskReward; select/move/edit/delete; Undo/Redo; Clear All confirm | Mode ro, Esc huy (D-02); handles ro (D-03); Fibo levels (D-09); magnet tat duoc (D-10) | persist versioned | D-01..D-11 | Pan/zoom/advance/reload van dinh (D-07) | Playwright | shots |
- REPLAY-010 | Trading BUY + T+2 inline | P0 | session index N | BUY 100 cp | Inline "T+1 - ve sau 1 phien" + cot Kha dung/Cho ve; SELL xam co ly do (khong chi tooltip) | available/pending tu SettlementEngine | T-02 | Tien ve som van chan | UAT T+2 | shot |
- REPLAY-011 | SL/TP + SHORT + sync | P0 | co position LONG + SHORT | Nhap SL/TP sai -> alert inline (khong bien mat); SL/TP dung -> preview % live; SHORT cong thuc dung dau; drawing RiskReward sync moi direction | sizing tren fill-est; net | T-02 | sizing null cam bien mat | Playwright matrix | shots |
- REPLAY-012 | Auto SL/TP + manual close + scoreboard | P0 | position mo | Step den TP -> auto close; mo moi -> close tay; check scoreboard/debrief/reset | PnL unrealized theo close; realized ngay; winrate/PF/debrief dung | net everywhere | T-04 | - | Playwright lab 01-09 | shots |
- REPLAY-013 | Pending LIMIT cancel | P0 | - | Dat LIMIT -> huy tu PendingOrders | Nut huy ro, trang thai cap nhat | DELETE/CANCEL 200 | T-05 | Khong co nut huy (gap cu) | Playwright | shot |
- REPLAY-014 | Signals per-candle | P1 | session | Mo SignalInspector + Explanation tai candle N | Giai thich khop candle N, version pin; khong don day BUY/SELL xuong fold | prefix observed<=cursor | R-01 | version lech | Playwright | shot |
- REPLAY-015 | Errors/offline | P1 | session | Tat BE / 422 co y | Toast phan loai + offline banner; khong [object Object]; ErrorBoundary co reset | envelope {code,message,field_errors} | observability | detail(str(e)) lo noi bo | Playwright | shot |

## 2. Scanner -> Replay

- SCAN-001 | Table + intent | P0 | co scan result | Click "Mo Replay" 1 dong | Sang /replay dung symbol/date/marker | intent API | flow | - | Playwright | shot |
- SCAN-002 | Filter/sort | P1 | nhieu dong | Loc + sort | Dung tap, khong mat trang thai | - | - | - | Playwright | shot |

## 3. Strategy Lab (5 tabs + single)

- LAB-001 | Catalog 72 signals | P1 | mo Lab | Duyet catalog, search | Mo ta + params ro | registry | - | - | Playwright | shot |
- LAB-002 | Builder->Battle prefill (critical) | P0 | builder co rule | Click "Dua sang Battle" | Sang tab battle + prefill Tester (strategy/cash/benchmark), khong mat rule | - | T-05 | Bien mat (gap cu) -> FAIL | Playwright | video |
- LAB-003 | Battle 3 strategies | P0 | YAML mau | Chay Battle MACD+RSI/EMA/Ichimoku | Equity 3 duong + star rating (winrate/sharpe/PF/MDD) | kernel next-open | parity | sweep lech batch | Playwright+pytest | shot+numbers |
- LAB-004 | Sweep params | P0 | 1 strategy + grid | Chay sweep | Ket qua == batch kernel cung input (parity) | sweep qua kernel | GAP-EXEC-01 | sweep legacy cu | pytest parity | csv |
- LAB-005 | Multiphase IS/OOS | P1 | battle xong | Chay IS/OOS | Split ro, chong overfit ghi chu | cutoff N-1 | - | - | Playwright | shot |
- LAB-006 | Flow BBViewer | P1 | mo flow | Xem BB default | Khong dut replay; horizons[H].bb_value map; N/A co ly do | envelope unify | - | doc FPT cam | Playwright | shot |
- LAB-007 | Assumptions footer | P1 | moi tab | Doc assumptions | Sweep/Battle/Single/Trade dong nhat 1 dong MarketProfile | profile | wording T+2 | T+2 vs T+2.5 lech | visual check | shots |

## 4. Backtest single (redirect)

- BACK-001 | Redirect + parity | P0 | - | Mo /backtest | Redirect /strategy-lab?tab=single + chu thich Single vs Battle | kernel | parity | orphan khong link | Playwright | shot |
- BACK-002 | Determinism | P0 | cung YAML+range | Chay 2 lan | Cung PnL/trades/fills | seed+cutoff | determinism | - | pytest 2-run | json diff |

## 5. Journal

- JOUR-001 | Lien ket + checklist | P0 | session co trades | Mo journal tu Replay (khong mat chart/replay) | Ten thong nhat; checklist_snapshot du | filtered | T-03 | 4 ten 2 khai niem | Playwright | shot |
- JOUR-002 | Debrief link | P1 | session xong | Mo debrief -> journal | So lieu khop scoreboard | net | - | - | Playwright | shot |

## 6. Analytics

- ANAL-001 | Metrics unify | P0 | co trades | Xem winrate/PF/Sharpe/MDD | Breakeven=not-winner; PF-inf -> N/A+tooltip; gate N>=30 warn; Sharpe 1 nghia | net | GAP-EXEC-05/06 | 3 truth cu | pytest+shot | numbers |
- ANAL-002 | Contracts required/optional | P1 | - | Reload nhieu lan | Khong 422/blank do dao required | snapshot | contracts | - | snapshot test | log |

## 7. Import / Sync

- IMP-001 | Preview | P0 | CafeF CSV/ZIP mau | Upload -> preview | Phan loai 6 nhan + checksum + stale warn; thieu --confirm khong commit | temp/tach commit | provenance | commit ngay (gap) | pytest+UI | preview json |
- IMP-002 | Accept + weekly + rollback | P0 | preview ok | Accept -> derive weekly -> rollback | INSERT + mutation + provenance; weekly VN_TRADING_WEEK_V1; rollback guard | atomic | - | loi giua flush->commit | pytest inject | db dump |
- IMP-003 | Anomaly weekend/x1000/date-reverse | P1 | file lech | Upload file sai gia x1000/nguoc date/weekend | Tu choi hoac canh bao ro | classifier | - | - | pytest | log |
- SYNC-001 | Stub blocked | P0 | provider ssi-stub | Execute sync mac dinh | Bi tu choi 403 + huong dan bat co; khong candle moi | default-deny | data-risk | ghi that (gap cu) | pytest | db count |
- SYNC-002 | Quarantine khi bat co | P1 | SUMI_ALLOW_SYNTHETIC=1 | Execute stub | Ghi quarantine + banner "du lieu gia", tach prod | is_synthetic | - | lan prod | pytest+UI | shot |

## Evidence moi run
- May: results json + screenshots pass+fail + console log + payload dumps (lookahead/BB) + hash prod + viewport 1440x1000 (+1280x800 khi cham layout).

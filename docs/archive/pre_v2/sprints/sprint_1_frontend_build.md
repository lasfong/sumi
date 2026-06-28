# Sprint 1 — Frontend Build Green

> **Mục tiêu:** `npm run build` pass 100% — KHÔNG có lỗi TypeScript.  
> **Thời gian:** 1–2 ngày  
> **Phụ thuộc:** Sprint 0  
> **Nguyên tắc:** KHÔNG thêm feature mới. CHỈ sửa lỗi compile.

> [!CAUTION]
> Sprint này chỉ sửa lỗi build. Không refactor, không thêm component, không đổi UI. Mục tiêu duy nhất là `npm run build` pass.

---

## S1-T1: Thống Nhất Type DrawingType

**Estimate:** 1–2 giờ

### Vấn đề

Hai file dùng type `DrawingType` khác nhau:
- `DrawingToolbar.tsx` dùng: `cursor | trendline | horizontal | fibonacci`
- `CandleChart.tsx` khai báo: `line | trendline | ray`
- Nhưng code trong CandleChart lại xử lý `horizontal` và `fibonacci`

→ TypeScript báo lỗi type không khớp.

### Files liên quan

- `frontend/src/components/chart/CandleChart.tsx`
- `frontend/src/components/chart/DrawingToolbar.tsx`

### Hướng dẫn từng bước

**Bước 1:** Mở file `DrawingToolbar.tsx`, tìm type `DrawingType` hiện tại. Ghi lại chính xác các giá trị.

**Bước 2:** Mở file `CandleChart.tsx`, tìm type `DrawingType` hoặc tương tự. Ghi lại chính xác.

**Bước 3:** Tạo file type chung hoặc sửa trực tiếp. Chọn MỘT nơi định nghĩa type và export:

```typescript
// Đặt trong DrawingToolbar.tsx hoặc tạo file riêng:
// frontend/src/types/drawing.ts

/** Tool đang active trên toolbar */
export type DrawingTool = 'cursor' | 'trendline' | 'horizontal' | 'fibonacci';

/** Type của drawing đã được vẽ lên chart (không bao gồm cursor vì cursor không tạo drawing) */
export type PersistedDrawingType = 'trendline' | 'horizontal' | 'fibonacci';

export interface DrawingLine {
  type: PersistedDrawingType;
  // ... các field khác giữ nguyên
}
```

**Bước 4:** Import type này ở cả `CandleChart.tsx` và `DrawingToolbar.tsx`:

```typescript
import { DrawingTool, PersistedDrawingType, DrawingLine } from '../../types/drawing';
```

**Bước 5:** Xóa các khai báo type cũ trong cả hai file.

**Bước 6:** Kiểm tra mọi chỗ dùng type này trong cả hai file, sửa cho khớp.

### Lệnh kiểm tra

```bash
cd frontend
npm run build
```

Xem còn lỗi liên quan đến `DrawingType` không.

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Type `DrawingTool` được định nghĩa một nơi duy nhất | Grep: `export type DrawingTool` chỉ xuất hiện 1 lần |
| Cả hai file dùng cùng type | Không còn lỗi TypeScript về DrawingType |
| Code logic vẫn xử lý horizontal, fibonacci | Đọc code confirm |

### Sai lầm thường gặp

- ❌ Đổi type thành `line | trendline | ray` mà không sửa code xử lý horizontal/fibonacci → runtime error
- ❌ Quên export type → file khác không import được
- ❌ Tạo type mới nhưng không xóa type cũ → confusing

---

## S1-T2: Destructure Đầy Đủ Props Trong CandleChart

**Estimate:** 30 phút – 1 giờ

### Vấn đề

Component `CandleChart` chỉ destructure `{ data, volumeData, markers }` nhưng bên trong code lại sử dụng `activeTool`, `drawings`, `onDrawingComplete` → TypeScript báo lỗi undefined.

### File liên quan

- `frontend/src/components/chart/CandleChart.tsx`

### Hướng dẫn từng bước

**Bước 1:** Tìm dòng khai báo component (tìm `forwardRef`):

```typescript
// TRƯỚC (sai):
export const CandleChart = forwardRef<CandleChartRef, CandleChartProps>(
  ({ data, volumeData, markers }, ref) => {
```

**Bước 2:** Thêm các props còn thiếu với giá trị mặc định:

```typescript
// SAU (đúng):
export const CandleChart = forwardRef<CandleChartRef, CandleChartProps>(
  ({ data, volumeData, markers, drawings = [], activeTool = 'cursor', onDrawingComplete }, ref) => {
```

**Bước 3:** Kiểm tra interface `CandleChartProps` đã có các field này chưa. Nếu chưa, thêm:

```typescript
interface CandleChartProps {
  data: CandleData[];
  volumeData: VolumeData[];
  markers?: SeriesMarker<Time>[];
  drawings?: DrawingLine[];
  activeTool?: DrawingTool;
  onDrawingComplete?: (drawing: DrawingLine) => void;
}
```

> [!WARNING]
> Dùng `?` (optional) cho props mới để không bắt buộc parent component phải truyền.

### Lệnh kiểm tra

```bash
cd frontend
npm run build
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Props destructured đầy đủ | Đọc dòng forwardRef, có đủ activeTool, drawings, onDrawingComplete |
| Interface CandleChartProps có đủ field | Đọc interface, confirm |
| Có giá trị mặc định cho optional props | `drawings = []`, `activeTool = 'cursor'` |
| Build không lỗi liên quan | `npm run build` không báo lỗi về CandleChart props |

### Sai lầm thường gặp

- ❌ Thêm props nhưng quên thêm vào interface → TypeScript vẫn lỗi
- ❌ Không cho giá trị mặc định → code crash khi parent không truyền
- ❌ Sai tên props (viết hoa/thường) → undefined

---

## S1-T3: Fix BarPrice/null Typing

**Estimate:** 30 phút – 1 giờ

### Vấn đề

`coordinateToPrice()` từ thư viện TradingView trả về `BarPrice | null`, nhưng code gán trực tiếp vào biến `number` → TypeScript lỗi.

### File liên quan

- `frontend/src/components/chart/CandleChart.tsx`

### Hướng dẫn từng bước

**Bước 1:** Tìm tất cả chỗ gọi `coordinateToPrice` trong file. Dùng Ctrl+F.

**Bước 2:** Với mỗi chỗ, thêm null check:

```typescript
// TRƯỚC (sai):
const price = mainSeries.coordinateToPrice(param.point.y);
// price có type BarPrice | null, nhưng code dùng như number

// SAU (đúng):
const rawPrice = mainSeries.coordinateToPrice(param.point.y);
if (rawPrice == null) return; // Early return nếu null
const price = Number(rawPrice); // Convert BarPrice thành number
```

**Bước 3:** Tìm tất cả chỗ lấy OHLC data từ `param.seriesData` và convert:

```typescript
// TRƯỚC (sai):
const ohlc = param.seriesData.get(mainSeries);
const high = ohlc.high; // BarPrice, không phải number

// SAU (đúng):
const ohlc = param.seriesData?.get(mainSeries);
if (!ohlc) return;
const high = Number((ohlc as any).high);
const low = Number((ohlc as any).low);
const open = Number((ohlc as any).open);
const close = Number((ohlc as any).close);
```

**Bước 4:** Kiểm tra hàm `getSnappedPrice()` nếu có — đảm bảo nhận và trả `number`.

### Lệnh kiểm tra

```bash
cd frontend
npm run build
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Không còn lỗi BarPrice \| null | `npm run build` pass phần này |
| Mọi coordinateToPrice đều có null check | Grep file, confirm |
| OHLC values được convert sang number | Đọc code confirm |

### Sai lầm thường gặp

- ❌ Dùng `as number` thay vì `Number()` → TypeScript cho qua nhưng có thể sai runtime
- ❌ Quên `return` sau null check → code tiếp tục chạy với giá trị undefined
- ❌ Dùng `!` (non-null assertion) → nguy hiểm, che giấu bug

---

## S1-T4: Fix MultiChartLayout Children Type

**Estimate:** 15–30 phút

### Vấn đề

Build báo: `children prop expects type ReactNode[] but only a single child was provided`.

### Files liên quan

- `frontend/src/components/layout/MultiChartLayout.tsx`
- `frontend/src/pages/ReplayPage.tsx`

### Hướng dẫn từng bước

**Bước 1:** Mở `MultiChartLayout.tsx`, tìm props interface:

```typescript
// TRƯỚC (sai):
interface MultiChartLayoutProps {
  children: React.ReactNode[];  // Bắt buộc phải truyền array
}
```

**Bước 2:** Sửa thành:

```typescript
// SAU (đúng):
interface MultiChartLayoutProps {
  children: React.ReactNode;  // Chấp nhận 1 hoặc nhiều children
}
```

**Bước 3:** Nếu component bên trong thực sự cần array (ví dụ `children.map()`), sửa logic:

```typescript
// Trong component body:
const childArray = React.Children.toArray(children);
// Dùng childArray thay cho children trực tiếp
```

### Lệnh kiểm tra

```bash
cd frontend
npm run build
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Props type là `React.ReactNode` (không có `[]`) | Đọc interface confirm |
| Nếu dùng .map thì dùng React.Children.toArray | Đọc code confirm |
| Build pass phần này | `npm run build` không lỗi MultiChartLayout |

---

## S1-T5: Dọn Unused Imports

**Estimate:** 15 phút

### Vấn đề

`ReplayPage.tsx` import `LineData` nhưng không dùng → TypeScript strict mode báo lỗi.

### File liên quan

- `frontend/src/pages/ReplayPage.tsx`

### Hướng dẫn từng bước

**Bước 1:** Mở file, tìm dòng import `LineData`:

```typescript
// Tìm dòng kiểu:
import { CandleData, LineData, ... } from 'lightweight-charts';
```

**Bước 2:** Xóa `LineData` khỏi import:

```typescript
// SAU:
import { CandleData, ... } from 'lightweight-charts';
```

**Bước 3:** Tìm thêm các import không dùng khác trong file (Ctrl+F hoặc dựa vào build errors).

> [!TIP]
> Cách nhanh nhất: nhìn build output, mọi dòng báo "is declared but its value is never read" đều là unused import cần xóa.

### Lệnh kiểm tra

```bash
cd frontend
npm run build
```

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| Không còn lỗi unused import | `npm run build` không báo "never read" |
| Không xóa import đang dùng | App vẫn build pass |

---

## S1-T6: Full Build Verification

**Estimate:** 30 phút – 1 giờ

### Mục tiêu

Sau khi hoàn thành S1-T1 đến S1-T5, chạy build lần cuối và sửa bất kỳ lỗi còn sót.

### Hướng dẫn

**Bước 1:** Chạy build:

```bash
cd frontend
npm run build
```

**Bước 2:** Nếu vẫn còn lỗi:
- Đọc từng lỗi TypeScript
- Phân loại: type mismatch, missing import, unused variable, etc.
- Sửa từng lỗi một, chạy build lại sau mỗi lần sửa

**Bước 3:** Khi build pass, ghi lại output:

```bash
npm run build
# Expected output cuối cùng:
# ✓ xxx modules transformed.
# dist/index.html  xxx
# dist/assets/...
# ✓ built in xxxms
```

**Bước 4:** Copy toàn bộ output thành công, paste vào báo cáo task.

### Lệnh kiểm tra CUỐI CÙNG

```bash
cd frontend
npm run build
```

**Kết quả bắt buộc:** Exit code 0, không có dòng `error` nào.

### Definition of Done

| Tiêu chí | Cách kiểm tra |
|----------|---------------|
| `npm run build` pass | Exit code 0, output có "built in" |
| Không có TypeScript error | Output không chứa "error TS" |
| Dist folder được tạo | `ls frontend/dist` có file |

---

## 🏁 Definition of Done — Sprint 1 Tổng Thể

```bash
cd E:\Workspace\sumi\frontend
npm run build
```

| # | Tiêu chí | Kết quả bắt buộc |
|---|----------|-------------------|
| 1 | Build exit code | 0 |
| 2 | TypeScript errors | 0 |
| 3 | Output cuối cùng | "built in xxxms" |
| 4 | dist/ folder | Tồn tại, có file |

> [!CAUTION]
> **Sprint 1 chỉ hoàn thành khi paste được output `npm run build` PASS vào báo cáo.**
> Không chấp nhận: "em thấy nó chạy được rồi", "em tắt strict mode", "em comment lỗi ra".

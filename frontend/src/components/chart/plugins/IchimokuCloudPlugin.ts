import type { 
    CustomData, 
    CustomSeriesOptions, 
    CustomSeriesPricePlotValues, 
    ICustomSeriesPaneRenderer, 
    ICustomSeriesPaneView, 
    PaneRendererCustomData, 
    Time, 
    PriceToCoordinateConverter 
} from 'lightweight-charts';
import type { CanvasRenderingTarget2D } from 'fancy-canvas';

export interface IchimokuCloudData extends CustomData<Time> {
    spanA?: number | null;
    spanB?: number | null;
}

export interface IchimokuCloudOptions extends CustomSeriesOptions {
    upColor: string;
    downColor: string;
}

export const defaultIchimokuOptions: Partial<IchimokuCloudOptions> = {
    upColor: 'rgba(38, 166, 154, 0.2)',
    downColor: 'rgba(239, 83, 80, 0.2)',
};

class IchimokuCloudRenderer implements ICustomSeriesPaneRenderer {
    _data: PaneRendererCustomData<Time, IchimokuCloudData> | null = null;
    _options: IchimokuCloudOptions | null = null;

    draw(target: CanvasRenderingTarget2D, priceConverter: PriceToCoordinateConverter): void {
        if (!this._data || !this._options || this._data.bars.length === 0) return;
        
        target.useMediaCoordinateSpace((scope) => {
            const ctx = scope.context;
            const bars = this._data!.bars;
            
            // To fill the area between Span A and Span B correctly, we need to iterate through the data
            // and draw polygons where they cross. For simplicity, we can draw small segments.
            
            for (let i = 1; i < bars.length; i++) {
                const prev = bars[i - 1];
                const curr = bars[i];
                
                const prevA = prev.originalData.spanA;
                const prevB = prev.originalData.spanB;
                const currA = curr.originalData.spanA;
                const currB = curr.originalData.spanB;
                
                if (
                    prevA === undefined || prevA === null || Number.isNaN(prevA) ||
                    prevB === undefined || prevB === null || Number.isNaN(prevB) ||
                    currA === undefined || currA === null || Number.isNaN(currA) ||
                    currB === undefined || currB === null || Number.isNaN(currB)
                ) {
                    continue;
                }

                const prevAy = priceConverter(prevA);
                const prevBy = priceConverter(prevB);
                const currAy = priceConverter(currA);
                const currBy = priceConverter(currB);
                
                if (prevAy === null || prevBy === null || currAy === null || currBy === null) continue;
                
                ctx.beginPath();
                ctx.moveTo(prev.x, prevAy);
                ctx.lineTo(curr.x, currAy);
                ctx.lineTo(curr.x, currBy);
                ctx.lineTo(prev.x, prevBy);
                ctx.closePath();
                
                // Color is based on which span is higher (lower y value means higher price)
                // If Span A > Span B (price), it's bullish
                ctx.fillStyle = currA > currB ? this._options!.upColor : this._options!.downColor;
                ctx.fill();
            }
        });
    }
}

export class IchimokuCloudPlugin implements ICustomSeriesPaneView<Time, IchimokuCloudData, IchimokuCloudOptions> {
    _renderer: IchimokuCloudRenderer;

    constructor() {
        this._renderer = new IchimokuCloudRenderer();
    }

    priceValueBuilder(plotRow: IchimokuCloudData): CustomSeriesPricePlotValues {
        return [plotRow.spanA ?? 0, plotRow.spanB ?? 0, plotRow.spanA ?? 0];
    }

    isWhitespace(data: IchimokuCloudData | CustomData): data is CustomData {
        return (data as IchimokuCloudData).spanA === undefined;
    }

    renderer(): ICustomSeriesPaneRenderer {
        return this._renderer;
    }

    update(data: PaneRendererCustomData<Time, IchimokuCloudData>, seriesOptions: IchimokuCloudOptions): void {
        this._renderer._data = data;
        this._renderer._options = seriesOptions;
    }

    defaultOptions(): IchimokuCloudOptions {
        return defaultIchimokuOptions as IchimokuCloudOptions;
    }
}

import { type ISeriesApi, type IPriceLine, LineStyle } from 'lightweight-charts';

export class PositionLineManager {
  private entryLine: IPriceLine | null = null;
  private stopLossLine: IPriceLine | null = null;
  private takeProfitLine: IPriceLine | null = null;
  private readonly series: ISeriesApi<'Candlestick'>;

  constructor(series: ISeriesApi<'Candlestick'>) {
    this.series = series;
  }

  update(
    position: { average_price: number },
    trade: { initial_stop_loss?: number | null; target_price?: number | null } | null
  ): void {
    // Entry
    if (this.entryLine) {
      this.entryLine.applyOptions({ price: position.average_price });
    } else {
      this.entryLine = this.series.createPriceLine({
        price: position.average_price,
        color: '#58A6FF',
        lineStyle: LineStyle.Dashed,
        title: 'Entry',
        axisLabelVisible: true,
      });
    }

    // Stop Loss
    if (trade?.initial_stop_loss != null) {
      if (this.stopLossLine) {
        this.stopLossLine.applyOptions({ price: trade.initial_stop_loss });
      } else {
        this.stopLossLine = this.series.createPriceLine({
          price: trade.initial_stop_loss,
          color: '#FF1744',
          lineStyle: LineStyle.Dashed,
          title: 'SL',
          axisLabelVisible: true,
        });
      }
    } else if (this.stopLossLine) {
      this.series.removePriceLine(this.stopLossLine);
      this.stopLossLine = null;
    }

    // Take Profit
    if (trade?.target_price != null) {
      if (this.takeProfitLine) {
        this.takeProfitLine.applyOptions({ price: trade.target_price });
      } else {
        this.takeProfitLine = this.series.createPriceLine({
          price: trade.target_price,
          color: '#00E676',
          lineStyle: LineStyle.Dashed,
          title: 'TP',
          axisLabelVisible: true,
        });
      }
    } else if (this.takeProfitLine) {
      this.series.removePriceLine(this.takeProfitLine);
      this.takeProfitLine = null;
    }
  }

  clear(): void {
    if (this.entryLine) {
      this.series.removePriceLine(this.entryLine);
      this.entryLine = null;
    }
    if (this.stopLossLine) {
      this.series.removePriceLine(this.stopLossLine);
      this.stopLossLine = null;
    }
    if (this.takeProfitLine) {
      this.series.removePriceLine(this.takeProfitLine);
      this.takeProfitLine = null;
    }
  }
}

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
    const entryTitle = `Entry: ${position.average_price.toLocaleString()}`;
    if (this.entryLine) {
      this.entryLine.applyOptions({ price: position.average_price, title: entryTitle });
    } else {
      this.entryLine = this.series.createPriceLine({
        price: position.average_price,
        color: '#58A6FF',
        lineStyle: LineStyle.Dashed,
        title: entryTitle,
        axisLabelVisible: true,
      });
    }

    // Stop Loss
    if (trade?.initial_stop_loss != null) {
      const slPct = position.average_price > 0
        ? ((trade.initial_stop_loss - position.average_price) / position.average_price) * 100
        : 0;
      const slTitle = `SL: ${trade.initial_stop_loss.toLocaleString()} (${slPct >= 0 ? '+' : ''}${slPct.toFixed(1)}%)`;
      if (this.stopLossLine) {
        this.stopLossLine.applyOptions({ price: trade.initial_stop_loss, title: slTitle });
      } else {
        this.stopLossLine = this.series.createPriceLine({
          price: trade.initial_stop_loss,
          color: '#FF1744',
          lineStyle: LineStyle.Dashed,
          title: slTitle,
          axisLabelVisible: true,
        });
      }
    } else if (this.stopLossLine) {
      this.series.removePriceLine(this.stopLossLine);
      this.stopLossLine = null;
    }

    // Take Profit
    if (trade?.target_price != null) {
      const tpPct = position.average_price > 0
        ? ((trade.target_price - position.average_price) / position.average_price) * 100
        : 0;
      const tpTitle = `TP: ${trade.target_price.toLocaleString()} (${tpPct >= 0 ? '+' : ''}${tpPct.toFixed(1)}%)`;
      if (this.takeProfitLine) {
        this.takeProfitLine.applyOptions({ price: trade.target_price, title: tpTitle });
      } else {
        this.takeProfitLine = this.series.createPriceLine({
          price: trade.target_price,
          color: '#00E676',
          lineStyle: LineStyle.Dashed,
          title: tpTitle,
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

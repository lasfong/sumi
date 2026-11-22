/**
 * Geometry helpers adapted from klinecharts-pro (Apache-2.0) and upgraded for KLineChart v10.1.1.
 */
import { utils } from 'klinecharts'
import type { Bounding, Chart, Coordinate, LineAttrs } from 'klinecharts'

export function getRotateCoordinate(
  coordinate: Coordinate,
  targetCoordinate: Coordinate,
  angle: number,
): Coordinate {
  const x =
    (coordinate.x - targetCoordinate.x) * Math.cos(angle) -
    (coordinate.y - targetCoordinate.y) * Math.sin(angle) +
    targetCoordinate.x
  const y =
    (coordinate.x - targetCoordinate.x) * Math.sin(angle) +
    (coordinate.y - targetCoordinate.y) * Math.cos(angle) +
    targetCoordinate.y
  return { x, y }
}

export function getRayLine(coordinates: Coordinate[], bounding: Bounding): LineAttrs[] {
  if (coordinates.length > 1) {
    let coordinate: Coordinate
    if (coordinates[0].x === coordinates[1].x && coordinates[0].y !== coordinates[1].y) {
      coordinate = {
        x: coordinates[0].x,
        y: coordinates[0].y < coordinates[1].y ? bounding.height : 0,
      }
    } else if (coordinates[0].x > coordinates[1].x) {
      coordinate = {
        x: 0,
        y: utils.getLinearYFromCoordinates(coordinates[0], coordinates[1], {
          x: 0,
          y: coordinates[0].y,
        }),
      }
    } else {
      coordinate = {
        x: bounding.width,
        y: utils.getLinearYFromCoordinates(coordinates[0], coordinates[1], {
          x: bounding.width,
          y: coordinates[0].y,
        }),
      }
    }
    return [{ coordinates: [coordinates[0], coordinate] }]
  }
  return []
}

export function getDistance(coordinate1: Coordinate, coordinate2: Coordinate): number {
  const xDis = Math.abs(coordinate1.x - coordinate2.x)
  const yDis = Math.abs(coordinate1.y - coordinate2.y)
  return Math.sqrt(xDis * xDis + yDis * yDis)
}

export function formatChartPrice(chart: Chart, value: number): string {
  const precision = chart.getSymbol()?.pricePrecision ?? 2
  const fixed = value.toFixed(precision)
  return chart.getDecimalFold().format(chart.getThousandsSeparator().format(fixed))
}

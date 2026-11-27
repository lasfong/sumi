/**
 * 17 Geometric / Fibonacci / Gann / Elliott Wave / Harmonic Overlays
 * Upgraded from klinecharts-pro (Apache-2.0) to strict KLineChart v10.1.1 OverlayTemplate API.
 */
import { utils } from 'klinecharts'
import type {
  CircleAttrs,
  Coordinate,
  LineAttrs,
  OverlayTemplate,
  PolygonAttrs,
  TextAttrs,
} from 'klinecharts'
import { formatChartPrice, getDistance, getRayLine, getRotateCoordinate } from './utils'

const FILL_BLUE = 'rgba(41, 98, 255, 0.14)'

export const arrow: OverlayTemplate = {
  name: 'arrow',
  totalStep: 3,
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  createPointFigures: ({ coordinates }) => {
    if (coordinates.length > 1) {
      const flag = coordinates[1].x > coordinates[0].x ? 0 : 1
      const kb = utils.getLinearSlopeIntercept(coordinates[0], coordinates[1])
      let offsetAngle: number
      if (kb) {
        offsetAngle = Math.atan(kb[0]) + Math.PI * flag
      } else {
        offsetAngle = coordinates[1].y > coordinates[0].y ? Math.PI / 2 : (Math.PI / 2) * 3
      }
      const r1 = getRotateCoordinate(
        { x: coordinates[1].x - 9, y: coordinates[1].y + 4.5 },
        coordinates[1],
        offsetAngle,
      )
      const r2 = getRotateCoordinate(
        { x: coordinates[1].x - 9, y: coordinates[1].y - 4.5 },
        coordinates[1],
        offsetAngle,
      )
      return [
        { type: 'line', attrs: { coordinates } },
        { type: 'line', ignoreEvent: true, attrs: { coordinates: [r1, coordinates[1], r2] } },
      ]
    }
    return []
  },
}

export const circle: OverlayTemplate = {
  name: 'circle',
  totalStep: 3,
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  styles: { circle: { color: FILL_BLUE } },
  createPointFigures: ({ coordinates }) => {
    if (coordinates.length > 1) {
      const radius = getDistance(coordinates[0], coordinates[1])
      return {
        type: 'circle',
        attrs: { ...coordinates[0], r: radius },
        styles: { style: 'stroke_fill' },
      }
    }
    return []
  },
}

export const rect: OverlayTemplate = {
  name: 'rect',
  totalStep: 3,
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  styles: { polygon: { color: FILL_BLUE } },
  createPointFigures: ({ coordinates }) => {
    if (coordinates.length > 1) {
      return [
        {
          type: 'polygon',
          attrs: {
            coordinates: [
              coordinates[0],
              { x: coordinates[1].x, y: coordinates[0].y },
              coordinates[1],
              { x: coordinates[0].x, y: coordinates[1].y },
            ],
          },
          styles: { style: 'stroke_fill' },
        },
      ]
    }
    return []
  },
}

export const triangle: OverlayTemplate = {
  name: 'triangle',
  totalStep: 4,
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  styles: { polygon: { color: FILL_BLUE } },
  createPointFigures: ({ coordinates }) => {
    return [
      {
        type: 'polygon',
        attrs: { coordinates },
        styles: { style: 'stroke_fill' },
      },
    ]
  },
}

export const parallelogram: OverlayTemplate = {
  name: 'parallelogram',
  totalStep: 4,
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  styles: { polygon: { color: FILL_BLUE } },
  createPointFigures: ({ coordinates }) => {
    if (coordinates.length === 2) {
      return [{ type: 'line', ignoreEvent: true, attrs: { coordinates } }]
    }
    if (coordinates.length === 3) {
      const p3 = {
        x: coordinates[0].x + (coordinates[2].x - coordinates[1].x),
        y: coordinates[2].y,
      }
      return [
        {
          type: 'polygon',
          attrs: { coordinates: [coordinates[0], coordinates[1], coordinates[2], p3] },
          styles: { style: 'stroke_fill' },
        },
      ]
    }
    return []
  },
  performEventPressedMove: ({ points, performPointIndex, performPoint }) => {
    if (performPointIndex < 2 && points[0] && points[1]) {
      points[0].value = performPoint.value
      points[1].value = performPoint.value
    }
  },
  performEventMoveForDrawing: ({ currentStep, points, performPoint }) => {
    if (currentStep === 2 && points[0]) {
      points[0].value = performPoint.value
    }
  },
}

export const fibonacciSegment: OverlayTemplate = {
  name: 'fibonacciSegment',
  totalStep: 3,
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  createPointFigures: ({ chart, coordinates, overlay }) => {
    const lines: LineAttrs[] = []
    const texts: TextAttrs[] = []
    const points = overlay.points
    if (
      coordinates.length > 1 &&
      typeof points[0]?.value === 'number' &&
      typeof points[1]?.value === 'number'
    ) {
      const textX = coordinates[1].x > coordinates[0].x ? coordinates[0].x : coordinates[1].x
      const percents = [1, 0.786, 0.618, 0.5, 0.382, 0.236, 0]
      const yDif = coordinates[0].y - coordinates[1].y
      const valueDif = points[0].value - points[1].value
      percents.forEach((percent) => {
        const y = coordinates[1].y + yDif * percent
        const price = formatChartPrice(chart, (points[1].value ?? 0) + valueDif * percent)
        lines.push({
          coordinates: [
            { x: coordinates[0].x, y },
            { x: coordinates[1].x, y },
          ],
        })
        texts.push({
          x: textX,
          y,
          text: `${price} (${(percent * 100).toFixed(1)}%)`,
          baseline: 'bottom',
        })
      })
    }
    return [
      { type: 'line', attrs: lines },
      { type: 'text', ignoreEvent: true, attrs: texts },
    ]
  },
}

export const fibonacciExtension: OverlayTemplate = {
  name: 'fibonacciExtension',
  totalStep: 4,
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  createPointFigures: ({ chart, coordinates, overlay }) => {
    const fbLines: LineAttrs[] = []
    const texts: TextAttrs[] = []
    const points = overlay.points
    if (
      coordinates.length > 2 &&
      typeof points[0]?.value === 'number' &&
      typeof points[1]?.value === 'number' &&
      typeof points[2]?.value === 'number'
    ) {
      const valueDif = points[1].value - points[0].value
      const yDif = coordinates[1].y - coordinates[0].y
      const percents = [0, 0.236, 0.382, 0.5, 0.618, 0.786, 1, 1.618]
      const textX = coordinates[2].x > coordinates[1].x ? coordinates[1].x : coordinates[2].x
      percents.forEach((percent) => {
        const y = coordinates[2].y + yDif * percent
        const price = formatChartPrice(chart, (points[2].value ?? 0) + valueDif * percent)
        fbLines.push({
          coordinates: [
            { x: coordinates[1].x, y },
            { x: coordinates[2].x, y },
          ],
        })
        texts.push({
          x: textX,
          y,
          text: `${price} (${(percent * 100).toFixed(1)}%)`,
          baseline: 'bottom',
        })
      })
    }
    return [
      { type: 'line', attrs: { coordinates }, styles: { style: 'dashed' } },
      { type: 'line', attrs: fbLines },
      { type: 'text', ignoreEvent: true, attrs: texts },
    ]
  },
}

export const fibonacciCircle: OverlayTemplate = {
  name: 'fibonacciCircle',
  totalStep: 3,
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  createPointFigures: ({ coordinates }) => {
    if (coordinates.length > 1) {
      const radius = getDistance(coordinates[0], coordinates[1])
      const percents = [0.236, 0.382, 0.5, 0.618, 0.786, 1]
      const circles: CircleAttrs[] = []
      const texts: TextAttrs[] = []
      percents.forEach((percent) => {
        const r = radius * percent
        circles.push({ ...coordinates[0], r })
        texts.push({
          x: coordinates[0].x,
          y: coordinates[0].y + r + 6,
          text: `${(percent * 100).toFixed(1)}%`,
        })
      })
      return [
        { type: 'circle', attrs: circles, styles: { style: 'stroke' } },
        { type: 'text', ignoreEvent: true, attrs: texts },
      ]
    }
    return []
  },
}

export const fibonacciSpiral: OverlayTemplate = {
  name: 'fibonacciSpiral',
  totalStep: 3,
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  createPointFigures: ({ coordinates, bounding }) => {
    if (coordinates.length > 1) {
      const startRadius = getDistance(coordinates[0], coordinates[1]) / Math.sqrt(24)
      const flag = coordinates[1].x > coordinates[0].x ? 0 : 1
      const kb = utils.getLinearSlopeIntercept(coordinates[0], coordinates[1])
      let offsetAngle: number
      if (kb) {
        offsetAngle = Math.atan(kb[0]) + Math.PI * flag
      } else {
        offsetAngle = coordinates[1].y > coordinates[0].y ? Math.PI / 2 : (Math.PI / 2) * 3
      }
      const rot1 = getRotateCoordinate(
        { x: coordinates[0].x - startRadius, y: coordinates[0].y },
        coordinates[0],
        offsetAngle,
      )
      const rot2 = getRotateCoordinate(
        { x: coordinates[0].x - startRadius, y: coordinates[0].y - startRadius },
        coordinates[0],
        offsetAngle,
      )
      const arcs = [
        { ...rot1, r: startRadius, startAngle: offsetAngle, endAngle: offsetAngle + Math.PI / 2 },
        {
          ...rot2,
          r: startRadius * 2,
          startAngle: offsetAngle + Math.PI / 2,
          endAngle: offsetAngle + Math.PI,
        },
      ]
      let x = coordinates[0].x - startRadius
      let y = coordinates[0].y - startRadius
      for (let i = 2; i < 9; i++) {
        const r = arcs[i - 2].r + arcs[i - 1].r
        let startAngle = 0
        switch (i % 4) {
          case 0:
            startAngle = offsetAngle
            x -= arcs[i - 2].r
            break
          case 1:
            startAngle = offsetAngle + Math.PI / 2
            y -= arcs[i - 2].r
            break
          case 2:
            startAngle = offsetAngle + Math.PI
            x += arcs[i - 2].r
            break
          case 3:
            startAngle = offsetAngle + (Math.PI / 2) * 3
            y += arcs[i - 2].r
            break
        }
        const rot = getRotateCoordinate({ x, y }, coordinates[0], offsetAngle)
        arcs.push({ ...rot, r, startAngle, endAngle: startAngle + Math.PI / 2 })
      }
      return [
        { type: 'arc', attrs: arcs },
        { type: 'line', attrs: getRayLine(coordinates, bounding) },
      ]
    }
    return []
  },
}

export const fibonacciSpeedResistanceFan: OverlayTemplate = {
  name: 'fibonacciSpeedResistanceFan',
  totalStep: 3,
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  createPointFigures: ({ coordinates, bounding }) => {
    const lines1: LineAttrs[] = []
    let lines2: LineAttrs[] = []
    const texts: TextAttrs[] = []
    if (coordinates.length > 1) {
      const xOffset = coordinates[1].x > coordinates[0].x ? -38 : 4
      const yOffset = coordinates[1].y > coordinates[0].y ? -2 : 20
      const xDistance = coordinates[1].x - coordinates[0].x
      const yDistance = coordinates[1].y - coordinates[0].y
      const percents = [1, 0.75, 0.618, 0.5, 0.382, 0.25, 0]
      percents.forEach((percent) => {
        const x = coordinates[1].x - xDistance * percent
        const y = coordinates[1].y - yDistance * percent
        lines1.push({ coordinates: [{ x, y: coordinates[0].y }, { x, y: coordinates[1].y }] })
        lines1.push({ coordinates: [{ x: coordinates[0].x, y }, { x: coordinates[1].x, y }] })
        lines2 = lines2.concat(getRayLine([coordinates[0], { x, y: coordinates[1].y }], bounding))
        lines2 = lines2.concat(getRayLine([coordinates[0], { x: coordinates[1].x, y }], bounding))
        texts.unshift({ x: coordinates[0].x + xOffset, y: y + 10, text: percent.toFixed(3) })
        texts.unshift({ x: x - 18, y: coordinates[0].y + yOffset, text: percent.toFixed(3) })
      })
    }
    return [
      { type: 'line', attrs: lines1 },
      { type: 'line', attrs: lines2 },
      { type: 'text', ignoreEvent: true, attrs: texts },
    ]
  },
}

export const gannBox: OverlayTemplate = {
  name: 'gannBox',
  totalStep: 3,
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  styles: { polygon: { color: FILL_BLUE } },
  createPointFigures: ({ coordinates }) => {
    if (coordinates.length > 1) {
      const quarterYDis = (coordinates[1].y - coordinates[0].y) / 4
      const xDis = coordinates[1].x - coordinates[0].x
      const dashedLines: LineAttrs[] = [
        { coordinates: [coordinates[0], { x: coordinates[1].x, y: coordinates[1].y - quarterYDis }] },
        { coordinates: [coordinates[0], { x: coordinates[1].x, y: coordinates[1].y - quarterYDis * 2 }] },
        { coordinates: [{ x: coordinates[0].x, y: coordinates[1].y }, { x: coordinates[1].x, y: coordinates[0].y + quarterYDis }] },
        { coordinates: [{ x: coordinates[0].x, y: coordinates[1].y }, { x: coordinates[1].x, y: coordinates[0].y + quarterYDis * 2 }] },
        { coordinates: [{ ...coordinates[0] }, { x: coordinates[0].x + xDis * 0.236, y: coordinates[1].y }] },
        { coordinates: [{ ...coordinates[0] }, { x: coordinates[0].x + xDis * 0.5, y: coordinates[1].y }] },
        { coordinates: [{ x: coordinates[0].x, y: coordinates[1].y }, { x: coordinates[0].x + xDis * 0.236, y: coordinates[0].y }] },
        { coordinates: [{ x: coordinates[0].x, y: coordinates[1].y }, { x: coordinates[0].x + xDis * 0.5, y: coordinates[0].y }] },
      ]
      const solidLines: LineAttrs[] = [
        { coordinates: [coordinates[0], coordinates[1]] },
        { coordinates: [{ x: coordinates[0].x, y: coordinates[1].y }, { x: coordinates[1].x, y: coordinates[0].y }] },
      ]
      return [
        {
          type: 'line',
          attrs: [
            { coordinates: [coordinates[0], { x: coordinates[1].x, y: coordinates[0].y }] },
            { coordinates: [{ x: coordinates[1].x, y: coordinates[0].y }, coordinates[1]] },
            { coordinates: [coordinates[1], { x: coordinates[0].x, y: coordinates[1].y }] },
            { coordinates: [{ x: coordinates[0].x, y: coordinates[1].y }, coordinates[0]] },
          ],
        },
        {
          type: 'polygon',
          ignoreEvent: true,
          attrs: {
            coordinates: [
              coordinates[0],
              { x: coordinates[1].x, y: coordinates[0].y },
              coordinates[1],
              { x: coordinates[0].x, y: coordinates[1].y },
            ],
          },
          styles: { style: 'fill' },
        },
        { type: 'line', attrs: dashedLines, styles: { style: 'dashed' } },
        { type: 'line', attrs: solidLines },
      ]
    }
    return []
  },
}

function makeWaveOverlay(name: string, totalStep: number): OverlayTemplate {
  return {
    name,
    totalStep,
    needDefaultPointFigure: true,
    needDefaultXAxisFigure: true,
    needDefaultYAxisFigure: true,
    createPointFigures: ({ coordinates }) => {
      const texts: TextAttrs[] = coordinates.map((coordinate, i) => ({
        ...coordinate,
        text: `(${i})`,
        baseline: 'bottom',
      }))
      return [
        { type: 'line', attrs: { coordinates } },
        { type: 'text', ignoreEvent: true, attrs: texts },
      ]
    },
  }
}

export const threeWaves = makeWaveOverlay('threeWaves', 5)
export const fiveWaves = makeWaveOverlay('fiveWaves', 7)
export const eightWaves = makeWaveOverlay('eightWaves', 10)
export const anyWaves = makeWaveOverlay('anyWaves', Number.MAX_SAFE_INTEGER)

export const abcd: OverlayTemplate = {
  name: 'abcd',
  totalStep: 5,
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  createPointFigures: ({ coordinates }) => {
    let acLine: Coordinate[] = []
    let bdLine: Coordinate[] = []
    const tags = ['A', 'B', 'C', 'D']
    const texts: TextAttrs[] = coordinates.map((coordinate, i) => ({
      ...coordinate,
      baseline: 'bottom',
      text: `(${tags[i] ?? i})`,
    }))
    if (coordinates.length > 2) {
      acLine = [coordinates[0], coordinates[2]]
      if (coordinates.length > 3) {
        bdLine = [coordinates[1], coordinates[3]]
      }
    }
    return [
      { type: 'line', attrs: { coordinates } },
      {
        type: 'line',
        attrs: [{ coordinates: acLine }, { coordinates: bdLine }],
        styles: { style: 'dashed' },
      },
      { type: 'text', ignoreEvent: true, attrs: texts },
    ]
  },
}

export const xabcd: OverlayTemplate = {
  name: 'xabcd',
  totalStep: 6,
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  styles: { polygon: { color: FILL_BLUE } },
  createPointFigures: ({ coordinates }) => {
    const dashedLines: LineAttrs[] = []
    const polygons: PolygonAttrs[] = []
    const tags = ['X', 'A', 'B', 'C', 'D']
    const texts: TextAttrs[] = coordinates.map((coordinate, i) => ({
      ...coordinate,
      baseline: 'bottom',
      text: `(${tags[i] ?? i})`,
    }))
    if (coordinates.length > 2) {
      dashedLines.push({ coordinates: [coordinates[0], coordinates[2]] })
      polygons.push({ coordinates: [coordinates[0], coordinates[1], coordinates[2]] })
      if (coordinates.length > 3) {
        dashedLines.push({ coordinates: [coordinates[1], coordinates[3]] })
        if (coordinates.length > 4) {
          dashedLines.push({ coordinates: [coordinates[2], coordinates[4]] })
          polygons.push({ coordinates: [coordinates[2], coordinates[3], coordinates[4]] })
        }
      }
    }
    return [
      { type: 'line', attrs: { coordinates } },
      { type: 'line', attrs: dashedLines, styles: { style: 'dashed' } },
      { type: 'polygon', ignoreEvent: true, attrs: polygons, styles: { style: 'fill' } },
      { type: 'text', ignoreEvent: true, attrs: texts },
    ]
  },
}

/** Wave 5: Multi-point directional Path polyline with terminal arrowhead. */
export const pathOverlay: OverlayTemplate = {
  name: 'path',
  totalStep: Number.MAX_SAFE_INTEGER,
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  createPointFigures: ({ coordinates }) => {
    if (coordinates.length < 2) {
      return [{ type: 'line', attrs: { coordinates } }]
    }
    const prev = coordinates[coordinates.length - 2]
    const last = coordinates[coordinates.length - 1]
    const angle = Math.atan2(last.y - prev.y, last.x - prev.x)
    const headLen = 10
    const leftWing: Coordinate = {
      x: last.x - headLen * Math.cos(angle - Math.PI / 6),
      y: last.y - headLen * Math.sin(angle - Math.PI / 6),
    }
    const rightWing: Coordinate = {
      x: last.x - headLen * Math.cos(angle + Math.PI / 6),
      y: last.y - headLen * Math.sin(angle + Math.PI / 6),
    }
    return [
      { type: 'line', attrs: { coordinates } },
      { type: 'line', ignoreEvent: true, attrs: { coordinates: [leftWing, last, rightWing] } },
    ]
  },
}

/** Wave 5: Wide semi-transparent Highlighter marker stroke. */
export const highlighter: OverlayTemplate = {
  name: 'highlighter',
  totalStep: Number.MAX_SAFE_INTEGER,
  needDefaultPointFigure: false,
  needDefaultXAxisFigure: false,
  needDefaultYAxisFigure: false,
  styles: {
    line: {
      color: 'rgba(251, 192, 45, 0.36)',
      size: 14,
      style: 'solid',
    },
  },
  createPointFigures: ({ coordinates }) => [
    {
      type: 'line',
      attrs: { coordinates },
      styles: {
        color: 'rgba(251, 192, 45, 0.36)',
        size: 14,
        style: 'solid',
      },
    },
  ],
}

/** Wave 5: Andrews Pitchfork — 3-point median line + parallel upper/lower channel tines. */
export const andrewsPitchfork: OverlayTemplate = {
  name: 'andrewsPitchfork',
  totalStep: 4,
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: true,
  styles: { polygon: { color: FILL_BLUE } },
  createPointFigures: ({ coordinates, bounding }) => {
    if (coordinates.length < 2) {
      return [{ type: 'line', attrs: { coordinates } }]
    }
    if (coordinates.length === 2) {
      return [{ type: 'line', attrs: { coordinates }, styles: { style: 'dashed' } }]
    }
    const [p0, p1, p2] = coordinates
    const mid: Coordinate = { x: (p1.x + p2.x) / 2, y: (p1.y + p2.y) / 2 }
    const dx = mid.x - p0.x
    const dy = mid.y - p0.y

    const medianRay = getRayLine([p0, mid], bounding)
    const upperRay = getRayLine([p1, { x: p1.x + dx, y: p1.y + dy }], bounding)
    const lowerRay = getRayLine([p2, { x: p2.x + dx, y: p2.y + dy }], bounding)

    const medianEnd = Array.isArray(medianRay) && medianRay.length === 2 ? medianRay[1] : mid
    const upperEnd = Array.isArray(upperRay) && upperRay.length === 2 ? upperRay[1] : p1
    const lowerEnd = Array.isArray(lowerRay) && lowerRay.length === 2 ? lowerRay[1] : p2

    return [
      {
        type: 'polygon',
        ignoreEvent: true,
        attrs: [
          { coordinates: [p1, upperEnd, medianEnd, mid] },
          { coordinates: [mid, medianEnd, lowerEnd, p2] },
        ],
        styles: { style: 'fill' },
      },
      { type: 'line', attrs: { coordinates: [p1, p2] }, styles: { style: 'dashed' } },
      { type: 'line', attrs: { coordinates: [p0, medianEnd] } },
      { type: 'line', attrs: { coordinates: [p1, upperEnd] } },
      { type: 'line', attrs: { coordinates: [p2, lowerEnd] } },
    ]
  },
}

const FIB_TIME_NUMBERS = [0, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89]

/** Wave 5: Fibonacci Time Zones — vertical time projection lines at Fibonacci bar intervals. */
export const fibTimeZones: OverlayTemplate = {
  name: 'fibTimeZones',
  totalStep: 3,
  needDefaultPointFigure: true,
  needDefaultXAxisFigure: true,
  needDefaultYAxisFigure: false,
  createPointFigures: ({ coordinates, bounding }) => {
    if (coordinates.length === 0) return []
    const x0 = coordinates[0].x
    const stepX = coordinates.length > 1 ? coordinates[1].x - x0 : 0
    if (Math.abs(stepX) < 1e-3) {
      return [
        {
          type: 'line',
          attrs: {
            coordinates: [
              { x: x0, y: 0 },
              { x: x0, y: bounding.height },
            ],
          },
        },
      ]
    }
    const lines: LineAttrs[] = []
    const texts: TextAttrs[] = []
    for (const fib of FIB_TIME_NUMBERS) {
      const x = x0 + fib * stepX
      if (x < -40 || x > bounding.width + 40) continue
      lines.push({
        coordinates: [
          { x, y: 0 },
          { x, y: bounding.height },
        ],
      })
      texts.push({
        x: x + 3,
        y: 14,
        text: `${fib}`,
        baseline: 'top',
      })
    }
    return [
      { type: 'line', attrs: lines, styles: { style: 'dashed' } },
      { type: 'text', ignoreEvent: true, attrs: texts },
    ]
  },
}

export const PRO_UPGRADED_OVERLAYS: OverlayTemplate[] = [
  arrow,
  circle,
  rect,
  triangle,
  parallelogram,
  fibonacciCircle,
  fibonacciSegment,
  fibonacciSpiral,
  fibonacciSpeedResistanceFan,
  fibonacciExtension,
  gannBox,
  threeWaves,
  fiveWaves,
  eightWaves,
  anyWaves,
  abcd,
  xabcd,
  pathOverlay,
  highlighter,
  andrewsPitchfork,
  fibTimeZones,
]

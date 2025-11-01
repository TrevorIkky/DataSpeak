"use client"

import { Scatter, ScatterChart, CartesianGrid, XAxis, YAxis } from "recharts";
import {
  ChartConfig,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  ChartLegend,
  ChartLegendContent,
} from "@/components/ui/chart";

interface ScatterChartRendererProps {
  data: Record<string, any>[];
  xAxis: string;
  yAxis: string;
}

export function ScatterChartRenderer({ data, xAxis, yAxis }: ScatterChartRendererProps) {
  const chartConfig: ChartConfig = {
    [yAxis]: {
      label: yAxis.charAt(0).toUpperCase() + yAxis.slice(1),
      color: "var(--chart-1)",
    },
  };

  return (
    <ChartContainer config={chartConfig} className="h-full w-full">
      <ScatterChart
        accessibilityLayer
        margin={{
          left: 12,
          right: 12,
        }}
      >
        <CartesianGrid vertical={false} />
        <XAxis
          type="number"
          dataKey={xAxis}
          name={xAxis}
          tickLine={false}
          axisLine={false}
          tickMargin={8}
        />
        <YAxis
          type="number"
          dataKey={yAxis}
          name={yAxis}
          tickLine={false}
          axisLine={false}
          tickMargin={8}
        />
        <ChartTooltip cursor={false} content={<ChartTooltipContent />} />
        <ChartLegend content={<ChartLegendContent />} />
        <Scatter
          name={`${xAxis} vs ${yAxis}`}
          data={data}
          fill={`var(--color-${yAxis})`}
        />
      </ScatterChart>
    </ChartContainer>
  );
}

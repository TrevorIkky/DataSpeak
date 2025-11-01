"use client"

import { Line, LineChart, CartesianGrid, XAxis } from "recharts";
import {
  ChartConfig,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  ChartLegend,
  ChartLegendContent,
} from "@/components/ui/chart";

interface LineChartRendererProps {
  data: Record<string, any>[];
  xAxis: string;
  yAxis: string[];
}

export function LineChartRenderer({ data, xAxis, yAxis }: LineChartRendererProps) {
  // Generate chart config from yAxis keys
  const chartConfig: ChartConfig = yAxis.reduce((config, key, index) => {
    config[key] = {
      label: key.charAt(0).toUpperCase() + key.slice(1),
      color: `var(--chart-${(index % 5) + 1})`,
    };
    return config;
  }, {} as ChartConfig);

  return (
    <ChartContainer config={chartConfig} className="h-full w-full">
      <LineChart
        accessibilityLayer
        data={data}
        margin={{ left: 12, right: 12 }}
      >
        <CartesianGrid vertical={false} />
        <XAxis
          dataKey={xAxis}
          tickLine={false}
          axisLine={false}
          tickMargin={8}
          tickFormatter={(value) => {
            const str = String(value);
            return str.length > 10 ? str.slice(0, 10) + "..." : str;
          }}
        />
        <ChartTooltip cursor={false} content={<ChartTooltipContent />} />
        {yAxis.length > 1 && <ChartLegend content={<ChartLegendContent />} />}
        {yAxis.map((key) => (
          <Line
            key={key}
            dataKey={key}
            type="natural"
            stroke={`var(--color-${key})`}
            strokeWidth={2}
            dot={false}
          />
        ))}
      </LineChart>
    </ChartContainer>
  );
}

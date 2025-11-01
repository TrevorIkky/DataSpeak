"use client"

import * as React from "react"
import { Label, PolarRadiusAxis, RadialBar, RadialBarChart } from "recharts";
import {
  ChartConfig,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from "@/components/ui/chart";

interface RadialChartRendererProps {
  data: Record<string, any>[];
  dataKey: string;
  nameKey: string;
}

export function RadialChartRenderer({ data, dataKey, nameKey }: RadialChartRendererProps) {
  // Generate chart config from data
  const chartConfig: ChartConfig = data.reduce((config, item, index) => {
    const name = String(item[nameKey]);
    config[name] = {
      label: name,
      color: `var(--chart-${(index % 5) + 1})`,
    };
    return config;
  }, {} as ChartConfig);

  // Transform data to include fill property
  const chartData = data.map((item, index) => ({
    ...item,
    fill: `var(--chart-${(index % 5) + 1})`,
  }));

  const totalValue = React.useMemo(() => {
    return chartData.reduce((acc, curr) => acc + (Number(curr[dataKey]) || 0), 0);
  }, [chartData, dataKey]);

  return (
    <ChartContainer config={chartConfig} className="h-full w-full">
      <RadialBarChart
        data={chartData}
        startAngle={-90}
        endAngle={380}
        innerRadius={30}
        outerRadius={110}
      >
        <ChartTooltip
          cursor={false}
          content={<ChartTooltipContent hideLabel nameKey={nameKey} />}
        />
        <PolarRadiusAxis tick={false} tickLine={false} axisLine={false}>
          <Label
            content={({ viewBox }) => {
              if (viewBox && "cx" in viewBox && "cy" in viewBox) {
                return (
                  <text
                    x={viewBox.cx}
                    y={viewBox.cy}
                    textAnchor="middle"
                    dominantBaseline="middle"
                  >
                    <tspan
                      x={viewBox.cx}
                      y={viewBox.cy}
                      className="fill-foreground text-3xl font-bold"
                    >
                      {totalValue.toLocaleString()}
                    </tspan>
                    <tspan
                      x={viewBox.cx}
                      y={(viewBox.cy || 0) + 24}
                      className="fill-muted-foreground"
                    >
                      Total
                    </tspan>
                  </text>
                );
              }
            }}
          />
        </PolarRadiusAxis>
        <RadialBar
          dataKey={dataKey}
          background
          cornerRadius={10}
        />
      </RadialBarChart>
    </ChartContainer>
  );
}

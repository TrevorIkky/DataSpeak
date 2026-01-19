import React, { useState, useMemo, memo, useCallback, Component, Suspense, type ErrorInfo, type ReactNode } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Download, Maximize2, Minimize2, AlertCircle, Loader2 } from "lucide-react";
import { cn } from "@/lib/utils";

// Lazy load Plot component to avoid loading full plotly.js at startup
import type { PlotParams } from "react-plotly.js";
const Plot = React.lazy(() => import("react-plotly.js"));

// Error Boundary for catching Plotly rendering errors
interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

class PlotlyErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("Plotly chart error:", error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return this.props.fallback || (
        <div className="flex items-center justify-center gap-2 h-[300px] text-destructive text-sm p-4 text-center">
          <AlertCircle className="h-5 w-5" />
          <span>Chart failed to render: {this.state.error?.message}</span>
        </div>
      );
    }

    return this.props.children;
  }
}

interface PlotlySandboxProps {
  /** Plotly data traces */
  plotlyData: Plotly.Data[];
  /** Plotly layout configuration */
  plotlyLayout: Partial<Plotly.Layout>;
  /** Chart title for display */
  title: string;
  /** Chart type (for metadata) */
  chartType: string;
  className?: string;
}

/**
 * Plotly chart renderer using react-plotly.js directly
 * Receives JSON data and layout from the backend - no code parsing needed
 */
export const PlotlySandbox = memo(function PlotlySandbox({
  plotlyData,
  plotlyLayout,
  title,
  chartType: _chartType,
  className,
}: PlotlySandboxProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const plotRef = useCallback((el: PlotParams | null) => {
    // Store reference for export functionality
    if (el) {
      (window as any).__plotlyChart = el;
    }
  }, []);

  const handleDownload = async () => {
    try {
      // Dynamically import Plotly only when needed for export
      const Plotly = await import('plotly.js');
      const chartElement = document.querySelector('.js-plotly-plot') as HTMLElement;
      if (chartElement) {
        const dataUrl = await Plotly.toImage(chartElement, {
          format: 'png',
          width: 1200,
          height: 600,
        });

        const link = document.createElement('a');
        link.download = `${title.replace(/\s+/g, '_')}.png`;
        link.href = dataUrl;
        link.click();
      }
    } catch (err) {
      console.error('Failed to download chart:', err);
    }
  };

  // Apply theme-aware colors to layout
  const themedLayout = useMemo(() => {
    return {
      ...plotlyLayout,
      paper_bgcolor: 'transparent',
      plot_bgcolor: 'transparent',
      font: {
        ...((plotlyLayout as any)?.font || {}),
        color: 'currentColor',
      },
    };
  }, [plotlyLayout]);

  const hasData = plotlyData && plotlyData.length > 0;

  return (
    <Card className={cn("overflow-hidden", className, isExpanded && "fixed inset-4 z-50")}>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium">{title}</CardTitle>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={handleDownload}
            title="Download as PNG"
          >
            <Download className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            onClick={() => setIsExpanded(!isExpanded)}
            title={isExpanded ? "Minimize" : "Expand"}
          >
            {isExpanded ? (
              <Minimize2 className="h-4 w-4" />
            ) : (
              <Maximize2 className="h-4 w-4" />
            )}
          </Button>
        </div>
      </CardHeader>
      <CardContent className="p-2">
        {hasData ? (
          <PlotlyErrorBoundary>
            <Suspense fallback={
              <div className="flex items-center justify-center gap-2 h-[350px] text-muted-foreground">
                <Loader2 className="h-5 w-5 animate-spin" />
                <span>Loading chart...</span>
              </div>
            }>
              <Plot
                data={plotlyData}
                layout={{
                  ...themedLayout,
                  autosize: true,
                  height: isExpanded ? undefined : 350,
                }}
                config={{
                  responsive: true,
                  displayModeBar: true,
                  displaylogo: false,
                  modeBarButtonsToRemove: ['lasso2d', 'select2d'],
                }}
                style={{ width: '100%', height: isExpanded ? 'calc(100vh - 10rem)' : '350px' }}
                useResizeHandler
                onInitialized={plotRef as any}
              />
            </Suspense>
          </PlotlyErrorBoundary>
        ) : (
          <div className="flex items-center justify-center gap-2 h-[300px] text-muted-foreground text-sm">
            <AlertCircle className="h-5 w-5" />
            <span>No chart data available</span>
          </div>
        )}
      </CardContent>
    </Card>
  );
});

export default PlotlySandbox;

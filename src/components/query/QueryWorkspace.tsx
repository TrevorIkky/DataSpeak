import { useEffect, useState, useRef } from "react";
import { QueryEditor } from "./QueryEditor";
import { EditableDataGrid } from "./EditableDataGrid";
import { TableDataTab } from "./TableDataTab";
import { ChartRenderer } from "@/components/visualization/ChartRenderer";
import { AiChatTab } from "@/components/ai/AiChatTab";
import { MapViewer } from "@/components/map/MapViewer";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { AlertCircle, Database, X, Plus, Maximize2, BarChart3, Table as TableIcon, MessageCircleMore, Code, MapPin } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { useIsMobile } from "@/hooks/use-mobile";
import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from "@/components/ui/resizable";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  DndContext,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
  DragEndEvent,
  DragOverlay,
  DragStartEvent,
} from '@dnd-kit/core';
import {
  SortableContext,
  useSortable,
  horizontalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { useQueryStore } from "@/stores/queryStore";
import { useConnectionStore } from "@/stores/connectionStore";
import { useUIStore } from "@/stores/uiStore";
import { useSchemaStore } from "@/stores/schemaStore";
import type { QueryTab, TableTab, VisualizationTab, Tab } from "@/types/query.types";
import type { DataGridChanges } from "@/types/datagrid.types";
import { extractTableFromQuery } from "@/lib/queryParser";
import { commitDataChanges } from "@/api/datagrid";
import { toast } from "sonner";

// Sortable Tab Component
interface SortableTabProps {
  tab: Tab;
  isActive: boolean;
  onSelect: () => void;
  onRemove: (e: React.MouseEvent) => void;
  isMobile: boolean;
}

function SortableTab({ tab, isActive, onSelect, onRemove, isMobile }: SortableTabProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: tab.id, disabled: isMobile });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0 : 1,
  };

  return (
    <button
      ref={setNodeRef}
      style={style}
      data-tab-id={tab.id}
      {...(isMobile ? {} : { ...attributes, ...listeners })}
      onClick={onSelect}
      className={`
        flex items-center gap-2 px-3 py-1.5 rounded-md text-sm transition-colors shrink-0
        ${!isMobile ? "touch-none" : ""}
        ${
          isActive
            ? "bg-primary/10 text-primary border border-primary/20"
            : "hover:bg-accent"
        }
        ${isDragging ? "cursor-grabbing" : isMobile ? "cursor-pointer" : "cursor-grab"}
      `}
    >
      <span className="truncate max-w-[150px]" title={tab.title}>{tab.title}</span>
      {tab.isLoading && (
        <div className="h-2 w-2 rounded-full bg-primary animate-pulse" />
      )}
      <button
        onClick={onRemove}
        className="hover:bg-destructive/20 rounded-sm p-0.5 transition-colors"
      >
        <X className="h-3 w-3" />
      </button>
    </button>
  );
}

export function QueryWorkspace() {
  const { activeConnection } = useConnectionStore();
  const { tabs, activeTabId, addTab, addChatTab, setActiveTab, removeTab, updateTab, reorderTabs } = useQueryStore();
  const { schema } = useSchemaStore();
  const {
    popoverOpen,
    setPopoverOpen,
    selectedGeography,
    setSelectedGeography,
    isMapFullscreen,
    setIsMapFullscreen,
    mobileQueryView,
    setMobileQueryView,
    mobileMapView,
    setMobileMapView,
    mobileChartView,
    setMobileChartView,
  } = useUIStore();

  const isMobile = useIsMobile();

  // Ref for tabs container to enable auto-scroll
  const tabsContainerRef = useRef<HTMLDivElement>(null);
  const prevTabCountRef = useRef(tabs.length);

  // State for drag overlay
  const [activeId, setActiveId] = useState<string | null>(null);

  // Configure drag sensors
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8, // 8px of movement required to start dragging
      },
    })
  );

  // Auto-scroll to new tab when added
  useEffect(() => {
    const wasTabAdded = tabs.length > prevTabCountRef.current;
    prevTabCountRef.current = tabs.length;

    if (wasTabAdded && tabsContainerRef.current) {
      // Scroll to end where new tab appears
      tabsContainerRef.current.scrollTo({
        left: tabsContainerRef.current.scrollWidth,
        behavior: 'smooth'
      });
    }
  }, [tabs.length]);

  const activeTab = tabs.find((t) => t.id === activeTabId);
  const hasChatTab = tabs.some((t) => t.type === 'chat');

  // Handle commit changes for query results
  const createQueryCommitHandler = (queryTab: QueryTab) => {
    return async (changes: DataGridChanges, originalRows: Record<string, any>[]) => {
      if (!activeConnection || !schema || !queryTab.query) {
        toast.error("Cannot commit changes: Missing connection or schema");
        return;
      }

      // Try to extract table name from query
      const tableInfo = extractTableFromQuery(queryTab.query);

      if (!tableInfo.isSimpleQuery || !tableInfo.tableName) {
        toast.error("Cannot commit changes: Only simple single-table queries are supported for commits");
        return;
      }

      // Find table in schema
      const table = schema.tables.find(t => t.name.toLowerCase() === tableInfo.tableName?.toLowerCase());

      if (!table) {
        toast.error(`Cannot commit changes: Table '${tableInfo.tableName}' not found in schema`);
        return;
      }

      // Extract primary key columns
      const primaryKeyColumns = table.columns
        .filter(col => col.is_primary_key)
        .map(col => col.name);

      if (primaryKeyColumns.length === 0) {
        toast.error(`Cannot commit changes: Table '${table.name}' has no primary key`);
        return;
      }

      try {
        const result = await commitDataChanges(
          activeConnection.id,
          table.name,
          primaryKeyColumns,
          changes,
          originalRows
        );

        toast.success(result.message);

        // Note: We don't auto-reload query results as the query might have filters/conditions
        // User can re-run the query manually to see changes
      } catch (error: any) {
        console.error("Failed to commit changes:", error);
        toast.error(`Failed to commit changes: ${error.message || error}`);
      }
    };
  };

  if (!activeConnection) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-8">
        <Database className="h-16 w-16 text-muted-foreground mb-4" />
        <h3 className="text-lg font-semibold mb-2">No Connection Selected</h3>
        <p className="text-sm text-muted-foreground text-center max-w-md">
          Select a database connection from the sidebar to start querying
        </p>
      </div>
    );
  }

  const renderTabContent = () => {
    if (!activeTab) {
      return (
        <Empty className="h-full">
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <Code />
            </EmptyMedia>
            <EmptyTitle>No Open Tabs</EmptyTitle>
            <EmptyDescription>
              Create a new query tab or select a table from the navigator to get started
            </EmptyDescription>
          </EmptyHeader>
          <EmptyContent>
            <Button onClick={() => addTab()} size="sm">
              <Plus className="h-4 w-4 mr-2" />
              New Query
            </Button>
          </EmptyContent>
        </Empty>
      );
    }

    if (activeTab.type === 'query') {
      const queryTab = activeTab as QueryTab;
      const hasVisualization = queryTab.showVisualization && queryTab.chartConfig;
      const hasMapView = selectedGeography !== null;

      // Try to extract table metadata from query
      let tableName: string | undefined;
      let primaryKeyColumns: string[] | undefined;

      if (queryTab.query && schema) {
        const tableInfo = extractTableFromQuery(queryTab.query);
        if (tableInfo.isSimpleQuery && tableInfo.tableName) {
          const table = schema.tables.find(t => t.name.toLowerCase() === tableInfo.tableName?.toLowerCase());
          if (table) {
            tableName = table.name;
            primaryKeyColumns = table.columns
              .filter(col => col.is_primary_key)
              .map(col => col.name);
          }
        }
      }

      // Mobile Layout: Tabbed Editor/Results
      if (isMobile) {
        return (
          <div className="flex flex-col h-full">
            {/* Mobile Tab Switcher */}
            <Tabs value={mobileQueryView} onValueChange={(v) => setMobileQueryView(v as 'editor' | 'results')} className="flex flex-col h-full">
              <div className="flex items-center justify-start px-4 py-2 border-b bg-card">
                <TabsList className="h-9">
                  <TabsTrigger value="editor" className="flex items-center gap-2 px-4">
                    <Code className="h-4 w-4" />
                    Editor
                  </TabsTrigger>
                  <TabsTrigger value="results" className="flex items-center gap-2 px-4">
                    <TableIcon className="h-4 w-4" />
                    Results
                    {queryTab.result && (
                      <span className="ml-1 text-xs text-muted-foreground">
                        ({queryTab.result.row_count})
                      </span>
                    )}
                  </TabsTrigger>
                </TabsList>
              </div>

              {/* Mobile Content */}
              <TabsContent value="editor" className="flex-1 overflow-hidden mt-0 data-[state=inactive]:hidden">
                <QueryEditor />
              </TabsContent>
              <TabsContent value="results" className="flex-1 overflow-hidden mt-0 data-[state=inactive]:hidden">
                <div className="h-full flex flex-col">
                  {queryTab.error ? (
                    <div className="p-4">
                      <Alert variant="destructive">
                        <AlertCircle className="h-4 w-4" />
                        <AlertDescription>{queryTab.error}</AlertDescription>
                      </Alert>
                    </div>
                  ) : queryTab.result ? (
                    hasMapView ? (
                      // Mobile Map View with tabs
                      <Tabs value={mobileMapView} onValueChange={(v) => setMobileMapView(v as 'grid' | 'map')} className="flex flex-col h-full">
                        <div className="flex items-center justify-between px-4 py-2 border-b bg-card">
                          <TabsList className="h-9">
                            <TabsTrigger value="grid" className="flex items-center gap-2 px-4">
                              <TableIcon className="h-4 w-4" />
                              Data
                            </TabsTrigger>
                            <TabsTrigger value="map" className="flex items-center gap-2 px-4">
                              <MapPin className="h-4 w-4" />
                              Map
                            </TabsTrigger>
                          </TabsList>
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => setSelectedGeography(null)}
                            className="h-9"
                          >
                            <X className="h-4 w-4" />
                          </Button>
                        </div>
                        <TabsContent value="grid" className="flex-1 overflow-hidden mt-0 data-[state=inactive]:hidden">
                          <EditableDataGrid
                            result={queryTab.result}
                            onGeographicCellClick={setSelectedGeography}
                            tableName={tableName}
                            primaryKeyColumns={primaryKeyColumns}
                            onCommitChanges={createQueryCommitHandler(queryTab)}
                          />
                        </TabsContent>
                        <TabsContent value="map" className="flex-1 overflow-hidden mt-0 data-[state=inactive]:hidden">
                          <MapViewer
                            geometry={selectedGeography.geometry}
                            columnName={selectedGeography.columnName}
                            rowIndex={selectedGeography.rowIndex}
                            onClose={() => setSelectedGeography(null)}
                            isFullscreen={false}
                          />
                        </TabsContent>
                      </Tabs>
                    ) : hasVisualization ? (
                      // Mobile Chart View with tabs
                      <Tabs value={mobileChartView} onValueChange={(v) => setMobileChartView(v as 'grid' | 'chart')} className="flex flex-col h-full">
                        <div className="flex items-center justify-start px-4 py-2 border-b bg-card">
                          <TabsList className="h-9">
                            <TabsTrigger value="grid" className="flex items-center gap-2 px-4">
                              <TableIcon className="h-4 w-4" />
                              Data
                            </TabsTrigger>
                            <TabsTrigger value="chart" className="flex items-center gap-2 px-4">
                              <BarChart3 className="h-4 w-4" />
                              Chart
                            </TabsTrigger>
                          </TabsList>
                        </div>
                        <TabsContent value="grid" className="flex-1 overflow-hidden mt-0 data-[state=inactive]:hidden">
                          <EditableDataGrid
                            result={queryTab.result}
                            onGeographicCellClick={setSelectedGeography}
                            tableName={tableName}
                            primaryKeyColumns={primaryKeyColumns}
                            onCommitChanges={createQueryCommitHandler(queryTab)}
                          />
                        </TabsContent>
                        <TabsContent value="chart" className="flex-1 overflow-hidden mt-0 data-[state=inactive]:hidden">
                          {queryTab.chartConfig && <ChartRenderer config={queryTab.chartConfig} data={queryTab.result} />}
                        </TabsContent>
                      </Tabs>
                    ) : (
                      <EditableDataGrid
                        result={queryTab.result}
                        onGeographicCellClick={setSelectedGeography}
                        tableName={tableName}
                        primaryKeyColumns={primaryKeyColumns}
                        onCommitChanges={createQueryCommitHandler(queryTab)}
                      />
                    )
                  ) : (
                    <div className="flex items-center justify-center h-full">
                      <p className="text-sm text-muted-foreground">
                        Execute a query to see results here
                      </p>
                    </div>
                  )}
                </div>
              </TabsContent>
            </Tabs>
          </div>
        );
      }

      // Desktop Layout: 40/60 split
      return (
        <div className="flex flex-col h-full">
          {/* Query Editor - Takes 40% */}
          <div className="h-[40%]">
            <QueryEditor />
          </div>

          {/* Results Area - Takes 60% */}
          <div className="h-[60%] flex flex-col">
            {queryTab.error ? (
              <div className="p-4">
                <Alert variant="destructive">
                  <AlertCircle className="h-4 w-4" />
                  <AlertDescription>{queryTab.error}</AlertDescription>
                </Alert>
              </div>
            ) : queryTab.result ? (
              <div className="flex flex-col h-full">
                {/* View Controls */}
                {(hasVisualization || hasMapView) && (
                  <div className="flex items-center justify-between px-4 py-2 border-b bg-card">
                    <h3 className="text-sm font-semibold">
                      {hasMapView ? "Results & Map" : "Results & Visualization"}
                    </h3>
                    <div className="flex items-center gap-2">
                      {hasVisualization && !hasMapView && (
                        <>
                          <Button
                            variant={!queryTab.showVisualization ? "default" : "ghost"}
                            size="sm"
                            onClick={() => updateTab(queryTab.id, { showVisualization: false })}
                            className="h-7"
                          >
                            <TableIcon className="h-4 w-4 mr-2" />
                            Grid Only
                          </Button>
                          <Button
                            variant={queryTab.showVisualization ? "default" : "ghost"}
                            size="sm"
                            onClick={() => updateTab(queryTab.id, { showVisualization: true })}
                            className="h-7"
                          >
                            <BarChart3 className="h-4 w-4 mr-2" />
                            Split View
                          </Button>
                        </>
                      )}
                      {hasMapView && (
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => setSelectedGeography(null)}
                          className="h-7"
                        >
                          <X className="h-4 w-4 mr-2" />
                          Close Map
                        </Button>
                      )}
                    </div>
                  </div>
                )}

                {/* Split View with Map, Chart, or Grid Only */}
                {hasMapView ? (
                  <ResizablePanelGroup direction="horizontal" className="flex-1">
                    <ResizablePanel
                      defaultSize={isMapFullscreen ? 0 : 50}
                      minSize={isMapFullscreen ? 0 : 30}
                      collapsible={true}
                    >
                      <EditableDataGrid
                        result={queryTab.result}
                        onGeographicCellClick={setSelectedGeography}
                        tableName={tableName}
                        primaryKeyColumns={primaryKeyColumns}
                        onCommitChanges={createQueryCommitHandler(queryTab)}
                      />
                    </ResizablePanel>
                    <ResizableHandle withHandle />
                    <ResizablePanel
                      defaultSize={isMapFullscreen ? 100 : 50}
                      minSize={isMapFullscreen ? 100 : 30}
                    >
                      <MapViewer
                        geometry={selectedGeography.geometry}
                        columnName={selectedGeography.columnName}
                        rowIndex={selectedGeography.rowIndex}
                        onClose={() => setSelectedGeography(null)}
                        isFullscreen={isMapFullscreen}
                        onToggleFullscreen={() => setIsMapFullscreen(!isMapFullscreen)}
                      />
                    </ResizablePanel>
                  </ResizablePanelGroup>
                ) : hasVisualization ? (
                  <ResizablePanelGroup direction="horizontal" className="flex-1">
                    <ResizablePanel defaultSize={50} minSize={30}>
                      <EditableDataGrid
                        result={queryTab.result}
                        onGeographicCellClick={setSelectedGeography}
                        tableName={tableName}
                        primaryKeyColumns={primaryKeyColumns}
                        onCommitChanges={createQueryCommitHandler(queryTab)}
                      />
                    </ResizablePanel>
                    <ResizableHandle withHandle />
                    <ResizablePanel defaultSize={50} minSize={30}>
                      {queryTab.chartConfig && <ChartRenderer config={queryTab.chartConfig} data={queryTab.result} />}
                    </ResizablePanel>
                  </ResizablePanelGroup>
                ) : (
                  <EditableDataGrid
                    result={queryTab.result}
                    onGeographicCellClick={setSelectedGeography}
                    onCommitChanges={async (changes) => {
                      console.log("Committing changes:", changes);
                      // TODO: Implement backend persistence
                    }}
                  />
                )}
              </div>
            ) : (
              <div className="flex items-center justify-center h-full">
                <p className="text-sm text-muted-foreground">
                  Execute a query to see results here
                </p>
              </div>
            )}
          </div>
        </div>
      );
    }

    if (activeTab.type === 'table') {
      const tableTab = activeTab as TableTab;
      return <TableDataTab tab={tableTab} />;
    }

    if (activeTab.type === 'visualization') {
      const vizTab = activeTab as VisualizationTab;
      return (
        <div className="flex flex-col h-full">
          {/* Visualization Controls */}
          <div className="flex items-center justify-between px-4 py-2 border-b bg-card">
            <h3 className="text-sm font-semibold">{vizTab.title}</h3>
            <div className="flex items-center gap-2">
              <Button
                variant={!vizTab.showGrid ? "default" : "ghost"}
                size="sm"
                onClick={() => updateTab(vizTab.id, { showGrid: false })}
                className="h-7"
              >
                <BarChart3 className="h-4 w-4 mr-2" />
                Chart Only
              </Button>
              <Button
                variant={vizTab.showGrid ? "default" : "ghost"}
                size="sm"
                onClick={() => updateTab(vizTab.id, { showGrid: true })}
                className="h-7"
              >
                <Maximize2 className="h-4 w-4 mr-2" />
                Split View
              </Button>
            </div>
          </div>

          {/* Content */}
          {vizTab.showGrid && vizTab.chartConfig ? (
            <ResizablePanelGroup direction="horizontal" className="flex-1">
              <ResizablePanel defaultSize={50} minSize={30}>
                <EditableDataGrid
                  result={vizTab.queryResult}
                  onCommitChanges={async (changes, originalRows) => {
                    console.log("Committing changes:", changes, "Original rows:", originalRows);
                    // Note: For arbitrary queries, we cannot commit changes without table metadata
                    // Commits are only supported when viewing tables directly
                  }}
                />
              </ResizablePanel>
              <ResizableHandle withHandle />
              <ResizablePanel defaultSize={50} minSize={30}>
                <ChartRenderer config={vizTab.chartConfig} data={vizTab.queryResult} />
              </ResizablePanel>
            </ResizablePanelGroup>
          ) : vizTab.chartConfig ? (
            <ChartRenderer config={vizTab.chartConfig} data={vizTab.queryResult} />
          ) : (
            <div className="flex items-center justify-center h-full">
              <p className="text-sm text-muted-foreground">No visualization config available</p>
            </div>
          )}
        </div>
      );
    }

    if (activeTab.type === 'chat') {
      return <AiChatTab />;
    }

    // This should never happen, but TypeScript needs this
    return (
      <div className="flex items-center justify-center h-full">
        <p className="text-sm text-muted-foreground">
          Unknown tab type
        </p>
      </div>
    );
  };

  // Handle drag start event
  const handleDragStart = (event: DragStartEvent) => {
    setActiveId(event.active.id as string);
  };

  // Handle drag end event
  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;

    if (over && active.id !== over.id) {
      const oldIndex = tabs.findIndex((tab) => tab.id === active.id);
      const newIndex = tabs.findIndex((tab) => tab.id === over.id);
      reorderTabs(oldIndex, newIndex);
    }

    setActiveId(null);
  };

  // Get the active dragged tab
  const draggedTab = activeId ? tabs.find((tab) => tab.id === activeId) : null;

  return (
    <div className="flex flex-col h-full">
      {/* Tabs Bar */}
      <div className="flex items-center border-b bg-card px-2 py-1 gap-1">
        <DndContext
          sensors={sensors}
          collisionDetection={closestCenter}
          onDragStart={handleDragStart}
          onDragEnd={handleDragEnd}
        >
          <SortableContext
            items={tabs.map((tab) => tab.id)}
            strategy={horizontalListSortingStrategy}
          >
            <div ref={tabsContainerRef} className="flex-1 flex items-center gap-1 overflow-x-auto scrollbar-none">
              {tabs.map((tab) => (
                <SortableTab
                  key={tab.id}
                  tab={tab}
                  isActive={tab.id === activeTabId}
                  onSelect={() => setActiveTab(tab.id)}
                  onRemove={(e) => {
                    e.stopPropagation();
                    removeTab(tab.id);
                  }}
                  isMobile={isMobile}
                />
              ))}
            </div>
          </SortableContext>
          <DragOverlay>
            {draggedTab ? (
              <div className="flex items-center gap-2 px-3 py-1.5 rounded-md text-sm bg-primary/10 text-primary border border-primary/20 cursor-grabbing shadow-lg">
                <span className="truncate max-w-[150px]" title={draggedTab.title}>{draggedTab.title}</span>
                {draggedTab.isLoading && (
                  <div className="h-2 w-2 rounded-full bg-primary animate-pulse" />
                )}
                <button className="hover:bg-destructive/20 rounded-sm p-0.5 transition-colors">
                  <X className="h-3 w-3" />
                </button>
              </div>
            ) : null}
          </DragOverlay>
        </DndContext>

        {hasChatTab ? (
          <button
            className="h-7 px-2 hover:bg-accent rounded-md transition-colors"
            onClick={() => addTab()}
            title="New SQL Query"
          >
            <Plus className="h-4 w-4" />
          </button>
        ) : (
          <Popover open={popoverOpen} onOpenChange={setPopoverOpen} modal={false}>
            <PopoverTrigger asChild>
              <button className="h-7 px-2 hover:bg-accent rounded-md transition-colors">
                <Plus className="h-4 w-4" />
              </button>
            </PopoverTrigger>
            <PopoverContent className="w-64 p-2 z-[100]" align="end" side="bottom" sideOffset={8}>
              <div className="space-y-1">
                <button
                  className="w-full flex items-start gap-2 px-2 py-2 hover:bg-accent rounded-md transition-colors text-left"
                  onClick={() => {
                    addTab();
                    setPopoverOpen(false);
                  }}
                >
                  <Code className="h-4 w-4 mt-0.5 shrink-0" />
                  <div>
                    <div className="font-medium text-sm">New SQL Query</div>
                    <div className="text-xs text-muted-foreground">Write and execute SQL</div>
                  </div>
                </button>
                <button
                  className="w-full flex items-start gap-2 px-2 py-2 hover:bg-accent rounded-md transition-colors text-left"
                  onClick={() => {
                    addChatTab();
                    setPopoverOpen(false);
                  }}
                >
                  <MessageCircleMore className="h-4 w-4 mt-0.5 shrink-0" />
                  <div>
                    <div className="font-medium text-sm">AI Assistant</div>
                    <div className="text-xs text-muted-foreground">Chat with AI about your data</div>
                  </div>
                </button>
              </div>
            </PopoverContent>
          </Popover>
        )}
      </div>

      {/* Tab Content */}
      <div className="flex-1 overflow-hidden">
        {renderTabContent()}
      </div>
    </div>
  );
}

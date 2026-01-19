import { useMemo, useEffect, useCallback } from "react";
import {
  ReactFlow,
  ReactFlowProvider,
  Node,
  Edge,
  Controls,
  ControlButton,
  Background,
  BackgroundVariant,
  useNodesState,
  useEdgesState,
  Handle,
  Position,
  BaseEdge,
  EdgeProps,
  getSmoothStepPath,
  useReactFlow,
  getNodesBounds,
  getViewportForBounds,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Network, Download } from "lucide-react";
import type { Table as TableType, Column } from "@/types/database.types";
import { toPng } from 'html-to-image';
import { save } from '@tauri-apps/plugin-dialog';
import { writeFile } from '@tauri-apps/plugin-fs';

interface ERDViewerProps {
  tables: TableType[];
  focusTableName: string;
}

type RelationshipType = 'one-to-one' | 'one-to-many' | 'many-to-many';

interface ERDEdgeData extends Record<string, unknown> {
  columnName: string;
  relationshipType: RelationshipType;
  isRelatedToFocus: boolean;
}

// Custom node component for table representation
function TableNode({ data }: { data: { label: string; columns: any[]; isFocused: boolean } }) {
  return (
    <div style={{ position: 'relative' }}>
      {/* Target handle - for incoming connections */}
      <Handle
        type="target"
        position={Position.Top}
        style={{ background: '#9333ea', width: 8, height: 8 }}
        isConnectable={false}
      />

      <Card className={`min-w-[250px] ${data.isFocused ? 'border-primary border-2 shadow-lg' : ''}`}>
        <CardHeader className="p-3">
          <CardTitle className="text-sm font-semibold flex items-center gap-2">
            <Network className="h-4 w-4" />
            {data.label}
            {data.isFocused && (
              <span className="ml-auto text-xs bg-primary text-primary-foreground px-2 py-0.5 rounded">
                Current
              </span>
            )}
          </CardTitle>
        </CardHeader>
        <CardContent className="p-3 pt-0">
          <div className="space-y-1 max-h-48 overflow-y-auto">
            {data.columns.slice(0, 10).map((col: any, idx: number) => (
              <div
                key={idx}
                className="text-xs flex items-center gap-1.5 py-1 border-b border-border last:border-0"
              >
                {col.is_primary_key && (
                  <span className="text-blue-500 font-bold" title="Primary Key">🔑</span>
                )}
                {col.is_foreign_key && (
                  <span className="text-purple-500 font-bold" title="Foreign Key">🔗</span>
                )}
                <span className={`font-mono ${col.is_primary_key || col.is_foreign_key ? 'font-semibold' : ''}`}>
                  {col.name}
                </span>
                <span className="text-muted-foreground ml-auto text-[10px]">
                  {col.data_type}
                </span>
              </div>
            ))}
            {data.columns.length > 10 && (
              <div className="text-xs text-muted-foreground italic text-center pt-1">
                +{data.columns.length - 10} more columns
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Source handle - for outgoing connections */}
      <Handle
        type="source"
        position={Position.Bottom}
        style={{ background: '#9333ea', width: 8, height: 8 }}
        isConnectable={false}
      />
    </div>
  );
}

// Custom edge component for ERD relationships
function ERDEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  style = {},
  data,
}: EdgeProps<Edge<ERDEdgeData>>) {
  const [edgePath] = getSmoothStepPath({
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  });

  const relationshipType = data?.relationshipType || 'one-to-many';
  const isRelatedToFocus = data?.isRelatedToFocus || false;

  // Determine marker IDs based on relationship type
  let markerEnd = '';
  let markerStart = '';

  if (relationshipType === 'one-to-one') {
    markerEnd = 'url(#one-marker)';
    markerStart = 'url(#one-marker)';
  } else if (relationshipType === 'one-to-many') {
    markerEnd = 'url(#many-marker)'; // Crow's foot on target (many side)
    markerStart = 'url(#one-marker)'; // Simple line on source (one side)
  } else if (relationshipType === 'many-to-many') {
    markerEnd = 'url(#many-marker)';
    markerStart = 'url(#many-marker)';
  }

  return (
    <BaseEdge
      id={id}
      path={edgePath}
      markerEnd={markerEnd}
      markerStart={markerStart}
      style={{
        ...style,
        stroke: isRelatedToFocus ? '#a855f7' : '#9333ea',
        strokeWidth: isRelatedToFocus ? 3 : 2,
      }}
    />
  );
}

const nodeTypes = {
  tableNode: TableNode,
};

const edgeTypes = {
  erdEdge: ERDEdge,
};

// Inner component that uses useReactFlow
function ERDViewerInner({ tables, focusTableName }: ERDViewerProps) {
  const { getNodes } = useReactFlow();

  // Download ERD as PNG
  const downloadImage = useCallback(async () => {
    const nodesBounds = getNodesBounds(getNodes());
    const viewport = getViewportForBounds(
      nodesBounds,
      1920, // width
      1080, // height
      0.5,  // min zoom
      2,    // max zoom
      0.2   // padding
    );

    const viewportElement = document.querySelector('.react-flow__viewport') as HTMLElement;

    if (viewportElement) {
      try {
        // Generate PNG data URL
        const dataUrl = await toPng(viewportElement, {
          backgroundColor: '#ffffff',
          width: 1920,
          height: 1080,
          style: {
            transform: `translate(${viewport.x}px, ${viewport.y}px) scale(${viewport.zoom})`,
          },
        });

        // Show save dialog
        const filePath = await save({
          defaultPath: `erd-${focusTableName}-${new Date().getTime()}.png`,
          filters: [{
            name: 'PNG Image',
            extensions: ['png']
          }]
        });

        if (filePath) {
          // Convert data URL to binary
          const base64Data = dataUrl.split(',')[1];
          const binaryData = Uint8Array.from(atob(base64Data), c => c.charCodeAt(0));

          // Write file
          await writeFile(filePath, binaryData);
        }
      } catch (error) {
        console.error('Error downloading ERD:', error);
      }
    }
  }, [getNodes, focusTableName]);

  // Build nodes and edges for ALL tables in the database
  const { initialNodes, initialEdges } = useMemo(() => {
    if (tables.length === 0) {
      return { initialNodes: [], initialEdges: [] };
    }

    const nodes: Node[] = [];
    const edges: Edge[] = [];

    // Calculate grid layout for all tables
    const cols = Math.ceil(Math.sqrt(tables.length));
    const horizontalSpacing = 400;
    const verticalSpacing = 350;

    // Create nodes for all tables
    tables.forEach((table, index) => {
      const col = index % cols;
      const row = Math.floor(index / cols);
      const x = col * horizontalSpacing + 100;
      const y = row * verticalSpacing + 100;

      nodes.push({
        id: table.name,
        type: 'tableNode',
        position: { x, y },
        data: {
          label: table.name,
          columns: table.columns,
          isFocused: table.name === focusTableName,
        },
      });
    });

    // Helper function to determine relationship type
    const determineRelationshipType = (sourceTable: TableType, fkColumn: Column): RelationshipType => {
      // If FK column is also a PK, it's likely one-to-one
      if (fkColumn.is_primary_key) {
        return 'one-to-one';
      }

      // Check if this is a junction table (many-to-many)
      // A junction table typically has multiple FKs that together form the PK
      const fkColumns = sourceTable.columns.filter(c => c.is_foreign_key);
      const pkColumns = sourceTable.columns.filter(c => c.is_primary_key);

      if (fkColumns.length >= 2 && pkColumns.length >= 2 &&
          fkColumns.every(fk => fk.is_primary_key)) {
        return 'many-to-many';
      }

      // Default to one-to-many
      return 'one-to-many';
    };

    // Create edges for all foreign key relationships
    tables.forEach((table) => {
      table.columns.forEach((col) => {
        if (col.is_foreign_key && col.foreign_key_table) {
          // Check if the referenced table exists in our tables list
          const targetExists = tables.some(t => t.name === col.foreign_key_table);

          if (targetExists) {
            const isRelatedToFocus = table.name === focusTableName || col.foreign_key_table === focusTableName;
            const relationshipType = determineRelationshipType(table, col);

            const edge: Edge<ERDEdgeData> = {
              id: `e-${table.name}-${col.foreign_key_table}-${col.name}`,
              source: table.name,
              target: col.foreign_key_table!,
              type: 'erdEdge',
              animated: isRelatedToFocus,
              data: {
                columnName: col.name,
                relationshipType,
                isRelatedToFocus,
              },
            };

            edges.push(edge);
          }
        }
      });
    });

    return { initialNodes: nodes, initialEdges: edges };
  }, [tables, focusTableName]);

  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);

  // Update nodes and edges when initialNodes/initialEdges change
  useEffect(() => {
    setNodes(initialNodes);
    setEdges(initialEdges);
  }, [initialNodes, initialEdges, setNodes, setEdges]);

  if (initialNodes.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-8">
        <Network className="h-12 w-12 text-muted-foreground mb-3" />
        <p className="text-sm text-muted-foreground">
          No tables found in schema
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* React Flow Canvas */}
      <div className="flex-1 border-t" style={{ width: '100%', height: '100%' }}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          nodeTypes={nodeTypes}
          edgeTypes={edgeTypes}
          fitView
          fitViewOptions={{ padding: 0.2 }}
          minZoom={0.1}
          maxZoom={2}
          nodesDraggable={true}
          nodesConnectable={false}
          elementsSelectable={true}
          edgesFocusable={true}
          deleteKeyCode={null}
          proOptions={{ hideAttribution: true }}
        >
          {/* Custom SVG Markers for ERD relationships */}
          <svg style={{ position: 'absolute', width: 0, height: 0 }}>
            <defs>
              {/* One side marker - simple perpendicular line */}
              <marker
                id="one-marker"
                markerWidth="12"
                markerHeight="12"
                refX="6"
                refY="6"
                orient="auto"
              >
                <line
                  x1="6"
                  y1="2"
                  x2="6"
                  y2="10"
                  stroke="#9333ea"
                  strokeWidth="2"
                />
              </marker>

              {/* Many side marker - crow's foot */}
              <marker
                id="many-marker"
                markerWidth="20"
                markerHeight="20"
                refX="10"
                refY="10"
                orient="auto"
              >
                <path
                  d="M 2,6 L 10,10 L 2,14 M 10,10 L 10,10"
                  fill="none"
                  stroke="#9333ea"
                  strokeWidth="2"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                />
              </marker>
            </defs>
          </svg>

          <Background variant={BackgroundVariant.Dots} gap={12} size={1} />
          <Controls>
            <ControlButton onClick={downloadImage} title="Download ERD">
              <Download className="h-4 w-4" />
            </ControlButton>
          </Controls>
        </ReactFlow>
      </div>

      {/* Legend and Info */}
      <div className="px-4 py-2 border-t bg-card safe-area-bottom">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-6 text-xs text-muted-foreground">
            <div className="flex items-center gap-2">
              <div className="h-5 w-5 border-2 border-primary bg-primary/10 rounded"></div>
              <span>Current Table</span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-blue-500 font-bold">🔑</span>
              <span>Primary Key</span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-purple-500 font-bold">🔗</span>
              <span>Foreign Key</span>
            </div>
            <div className="flex items-center gap-2">
              <svg width="32" height="12" viewBox="0 0 32 12">
                <line x1="0" y1="6" x2="24" y2="6" stroke="#9333ea" strokeWidth="2" />
                <line x1="24" y1="2" x2="24" y2="10" stroke="#9333ea" strokeWidth="2" />
                <path d="M 6,2 L 14,6 L 6,10" fill="none" stroke="#9333ea" strokeWidth="2" />
              </svg>
              <span>One-to-Many</span>
            </div>
            <div className="flex items-center gap-2">
              <svg width="32" height="12" viewBox="0 0 32 12">
                <line x1="0" y1="6" x2="32" y2="6" stroke="#9333ea" strokeWidth="2" />
                <line x1="0" y1="2" x2="0" y2="10" stroke="#9333ea" strokeWidth="2" />
                <line x1="32" y1="2" x2="32" y2="10" stroke="#9333ea" strokeWidth="2" />
              </svg>
              <span>One-to-One</span>
            </div>
            <div className="flex items-center gap-2">
              <svg width="32" height="12" viewBox="0 0 32 12">
                <line x1="0" y1="6" x2="32" y2="6" stroke="#9333ea" strokeWidth="2" />
                <path d="M 6,2 L 14,6 L 6,10" fill="none" stroke="#9333ea" strokeWidth="2" />
                <path d="M 18,2 L 26,6 L 18,10" fill="none" stroke="#9333ea" strokeWidth="2" />
              </svg>
              <span>Many-to-Many</span>
            </div>
          </div>
          <div className="text-xs text-muted-foreground">
            {nodes.length} {nodes.length === 1 ? 'table' : 'tables'} · {edges.length} {edges.length === 1 ? 'relationship' : 'relationships'}
          </div>
        </div>
      </div>
    </div>
  );
}

// Outer component with ReactFlowProvider
export function ERDViewer(props: ERDViewerProps) {
  return (
    <ReactFlowProvider>
      <ERDViewerInner {...props} />
    </ReactFlowProvider>
  );
}

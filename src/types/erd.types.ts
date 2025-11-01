export type ERDNode = {
  id: string;
  type: string;
  data: {
    label: string;
    columns: {
      name: string;
      type: string;
      isPrimaryKey: boolean;
      isForeignKey: boolean;
    }[];
  };
  position: { x: number; y: number };
};

export type ERDEdge = {
  id: string;
  source: string;
  target: string;
  sourceHandle?: string;
  targetHandle?: string;
  type?: string;
  label?: string;
};

export type ERDData = {
  nodes: ERDNode[];
  edges: ERDEdge[];
};

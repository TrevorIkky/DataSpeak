// Geographic data types for PostGIS/MySQL Spatial support

export type GeometryType =
  | "Point"
  | "LineString"
  | "Polygon"
  | "MultiPoint"
  | "MultiLineString"
  | "MultiPolygon"
  | "GeometryCollection";

export interface Coordinate {
  lat: number;
  lng: number;
}

export interface PointGeometry {
  type: "Point";
  coordinates: Coordinate;
}

export interface LineStringGeometry {
  type: "LineString";
  coordinates: Coordinate[];
}

export interface PolygonGeometry {
  type: "Polygon";
  coordinates: Coordinate[][];
}

export interface MultiPointGeometry {
  type: "MultiPoint";
  coordinates: Coordinate[];
}

export interface MultiLineStringGeometry {
  type: "MultiLineString";
  coordinates: Coordinate[][];
}

export interface MultiPolygonGeometry {
  type: "MultiPolygon";
  coordinates: Coordinate[][][];
}

export interface GeometryCollectionGeometry {
  type: "GeometryCollection";
  geometries: GeometryData[];
}

export type GeometryData =
  | PointGeometry
  | LineStringGeometry
  | PolygonGeometry
  | MultiPointGeometry
  | MultiLineStringGeometry
  | MultiPolygonGeometry
  | GeometryCollectionGeometry;

export interface GeographicCell {
  columnName: string;
  rowIndex: number;
  geometry: GeometryData;
  rawValue: string;
}

import type { GeometryData, Coordinate } from "@/types/geography.types";

/**
 * Parse WKT (Well-Known Text) format to GeoJSON-like geometry
 * Supports: POINT, LINESTRING, POLYGON, MULTIPOINT, MULTILINESTRING, MULTIPOLYGON
 */
export function parseWKT(wkt: string): GeometryData | null {
  if (!wkt || typeof wkt !== "string") return null;

  // Remove SRID prefix if present (e.g., "SRID=4326;POINT(...")
  const wktClean = wkt.replace(/^SRID=\d+;/, "").trim();

  try {
    // POINT
    if (wktClean.startsWith("POINT")) {
      const coords = extractCoordinates(wktClean);
      if (coords.length === 1) {
        return {
          type: "Point",
          coordinates: coords[0],
        };
      }
    }

    // LINESTRING
    if (wktClean.startsWith("LINESTRING")) {
      const coords = extractCoordinates(wktClean);
      return {
        type: "LineString",
        coordinates: coords,
      };
    }

    // POLYGON
    if (wktClean.startsWith("POLYGON")) {
      const rings = extractPolygonRings(wktClean);
      return {
        type: "Polygon",
        coordinates: rings,
      };
    }

    // MULTIPOINT
    if (wktClean.startsWith("MULTIPOINT")) {
      const coords = extractMultiPointCoordinates(wktClean);
      return {
        type: "MultiPoint",
        coordinates: coords,
      };
    }

    // MULTILINESTRING
    if (wktClean.startsWith("MULTILINESTRING")) {
      const lines = extractMultiLineStringCoordinates(wktClean);
      return {
        type: "MultiLineString",
        coordinates: lines,
      };
    }

    // MULTIPOLYGON
    if (wktClean.startsWith("MULTIPOLYGON")) {
      const polygons = extractMultiPolygonCoordinates(wktClean);
      return {
        type: "MultiPolygon",
        coordinates: polygons,
      };
    }

    return null;
  } catch (error) {
    console.error("Failed to parse WKT:", error);
    return null;
  }
}

/**
 * Extract coordinates from WKT string
 * Returns array of {lat, lng} objects
 */
function extractCoordinates(wkt: string): Coordinate[] {
  // Extract content between parentheses
  const match = wkt.match(/\(([^)]+)\)/);
  if (!match) return [];

  const coordStr = match[1];
  const pairs = coordStr.split(",").map((s) => s.trim());

  return pairs.map((pair) => {
    const [lng, lat] = pair.split(/\s+/).map(Number);
    return { lat, lng };
  });
}

/**
 * Extract polygon rings (outer + holes)
 */
function extractPolygonRings(wkt: string): Coordinate[][] {
  // POLYGON((outer),(hole1),(hole2))
  const matches = wkt.matchAll(/\(([^()]+)\)/g);
  const rings: Coordinate[][] = [];

  for (const match of matches) {
    const coordStr = match[1];
    const pairs = coordStr.split(",").map((s) => s.trim());
    const ring = pairs.map((pair) => {
      const [lng, lat] = pair.split(/\s+/).map(Number);
      return { lat, lng };
    });
    rings.push(ring);
  }

  return rings;
}

/**
 * Extract MULTIPOINT coordinates
 * Format: MULTIPOINT((x1 y1),(x2 y2)) or MULTIPOINT(x1 y1,x2 y2)
 */
function extractMultiPointCoordinates(wkt: string): Coordinate[] {
  // Remove MULTIPOINT wrapper
  const content = wkt.replace(/^MULTIPOINT\s*\(/, "").replace(/\)$/, "");

  // Handle both formats: ((x y),(x y)) and (x y,x y)
  const coords: Coordinate[] = [];

  if (content.includes("),(")) {
    // Format: ((x1 y1),(x2 y2))
    const points = content.split("),(");
    points.forEach((point) => {
      const clean = point.replace(/[()]/g, "");
      const [lng, lat] = clean.split(/\s+/).map(Number);
      coords.push({ lat, lng });
    });
  } else {
    // Format: (x1 y1,x2 y2)
    const pairs = content.split(",").map((s) => s.trim());
    pairs.forEach((pair) => {
      const [lng, lat] = pair.split(/\s+/).map(Number);
      coords.push({ lat, lng });
    });
  }

  return coords;
}

/**
 * Extract MULTILINESTRING coordinates
 */
function extractMultiLineStringCoordinates(wkt: string): Coordinate[][] {
  const lines: Coordinate[][] = [];
  const matches = wkt.matchAll(/\(([^()]+)\)/g);

  for (const match of matches) {
    const coordStr = match[1];
    const pairs = coordStr.split(",").map((s) => s.trim());
    const line = pairs.map((pair) => {
      const [lng, lat] = pair.split(/\s+/).map(Number);
      return { lat, lng };
    });
    lines.push(line);
  }

  return lines;
}

/**
 * Extract MULTIPOLYGON coordinates
 */
function extractMultiPolygonCoordinates(wkt: string): Coordinate[][][] {
  const polygons: Coordinate[][][] = [];

  // Match each polygon: ((outer),(hole))
  const polygonMatches = wkt.matchAll(/\(\(([^)]+(?:\),[^)]+)*)\)\)/g);

  for (const polygonMatch of polygonMatches) {
    const polygonContent = polygonMatch[1];
    const rings: Coordinate[][] = [];

    // Match each ring within the polygon
    const ringMatches = polygonContent.matchAll(/\(([^)]+)\)/g);
    for (const ringMatch of ringMatches) {
      const coordStr = ringMatch[1];
      const pairs = coordStr.split(",").map((s) => s.trim());
      const ring = pairs.map((pair) => {
        const [lng, lat] = pair.split(/\s+/).map(Number);
        return { lat, lng };
      });
      rings.push(ring);
    }

    polygons.push(rings);
  }

  return polygons;
}

/**
 * Check if a string looks like a WKT geometry
 */
export function isWKTGeometry(value: string): boolean {
  if (!value || typeof value !== "string") return false;

  const wktPattern =
    /^(SRID=\d+;)?(POINT|LINESTRING|POLYGON|MULTIPOINT|MULTILINESTRING|MULTIPOLYGON|GEOMETRYCOLLECTION)\s*\(/i;
  return wktPattern.test(value);
}

/**
 * Get center coordinate from geometry for map centering
 */
export function getGeometryCenter(geometry: GeometryData): Coordinate {
  switch (geometry.type) {
    case "Point":
      return geometry.coordinates;

    case "LineString":
    case "MultiPoint":
      return getCenterOfCoordinates(geometry.coordinates);

    case "Polygon":
      return getCenterOfCoordinates(geometry.coordinates[0]);

    case "MultiLineString":
      return getCenterOfCoordinates(geometry.coordinates.flat());

    case "MultiPolygon":
      return getCenterOfCoordinates(geometry.coordinates.flat(2));

    case "GeometryCollection":
      // Get center of first geometry
      if (geometry.geometries.length > 0) {
        return getGeometryCenter(geometry.geometries[0]);
      }
      return { lat: 0, lng: 0 };

    default:
      return { lat: 0, lng: 0 };
  }
}

/**
 * Calculate center of array of coordinates
 */
function getCenterOfCoordinates(coords: Coordinate[]): Coordinate {
  if (coords.length === 0) return { lat: 0, lng: 0 };

  const sum = coords.reduce(
    (acc, coord) => ({
      lat: acc.lat + coord.lat,
      lng: acc.lng + coord.lng,
    }),
    { lat: 0, lng: 0 }
  );

  return {
    lat: sum.lat / coords.length,
    lng: sum.lng / coords.length,
  };
}

/**
 * Get bounds from geometry for map fitting
 */
export function getGeometryBounds(geometry: GeometryData): {
  north: number;
  south: number;
  east: number;
  west: number;
} | null {
  const allCoords = getAllCoordinates(geometry);
  if (allCoords.length === 0) return null;

  let north = -90,
    south = 90,
    east = -180,
    west = 180;

  allCoords.forEach((coord) => {
    north = Math.max(north, coord.lat);
    south = Math.min(south, coord.lat);
    east = Math.max(east, coord.lng);
    west = Math.min(west, coord.lng);
  });

  return { north, south, east, west };
}

/**
 * Get all coordinates from geometry (flattened)
 */
function getAllCoordinates(geometry: GeometryData): Coordinate[] {
  switch (geometry.type) {
    case "Point":
      return [geometry.coordinates];

    case "LineString":
    case "MultiPoint":
      return geometry.coordinates;

    case "Polygon":
      return geometry.coordinates.flat();

    case "MultiLineString":
      return geometry.coordinates.flat();

    case "MultiPolygon":
      return geometry.coordinates.flat(2);

    case "GeometryCollection":
      return geometry.geometries.flatMap(getAllCoordinates);

    default:
      return [];
  }
}

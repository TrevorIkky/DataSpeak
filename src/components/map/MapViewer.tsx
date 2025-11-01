import { useEffect, useMemo } from "react";
import { MapContainer, TileLayer, Marker, Polyline, Polygon, Popup, useMap } from "react-leaflet";
import { Icon, LatLngBounds } from "leaflet";
import "leaflet/dist/leaflet.css";
import type { GeometryData } from "@/types/geography.types";
import { getGeometryCenter, getGeometryBounds } from "@/lib/geoUtils";
import { Button } from "@/components/ui/button";
import { X, Maximize2, Minimize2, MapPin } from "lucide-react";
import { Badge } from "@/components/ui/badge";

interface MapViewerProps {
  geometry: GeometryData;
  columnName?: string;
  rowIndex?: number;
  onClose: () => void;
  isFullscreen?: boolean;
  onToggleFullscreen?: () => void;
}

// Fix Leaflet default icon issue with bundlers
const defaultIcon = new Icon({
  iconUrl: "https://unpkg.com/leaflet@1.9.4/dist/images/marker-icon.png",
  iconRetinaUrl: "https://unpkg.com/leaflet@1.9.4/dist/images/marker-icon-2x.png",
  shadowUrl: "https://unpkg.com/leaflet@1.9.4/dist/images/marker-shadow.png",
  iconSize: [25, 41],
  iconAnchor: [12, 41],
  popupAnchor: [1, -34],
  shadowSize: [41, 41],
});

// Component to fit bounds when geometry changes
function FitBounds({ geometry }: { geometry: GeometryData }) {
  const map = useMap();

  useEffect(() => {
    const bounds = getGeometryBounds(geometry);
    if (bounds) {
      const leafletBounds = new LatLngBounds(
        [bounds.south, bounds.west],
        [bounds.north, bounds.east]
      );
      map.fitBounds(leafletBounds, { padding: [50, 50], maxZoom: 16 });
    } else {
      const center = getGeometryCenter(geometry);
      map.setView([center.lat, center.lng], 13);
    }
  }, [geometry, map]);

  return null;
}

export function MapViewer({
  geometry,
  columnName,
  rowIndex,
  onClose,
  isFullscreen = false,
  onToggleFullscreen,
}: MapViewerProps) {
  const center = useMemo(() => getGeometryCenter(geometry), [geometry]);

  // Render geometry based on type
  const renderGeometry = (geom: GeometryData) => {
    switch (geom.type) {
      case "Point":
        return (
          <Marker position={[geom.coordinates.lat, geom.coordinates.lng]} icon={defaultIcon}>
            <Popup>
              <div className="text-sm">
                <strong>Point</strong>
                <br />
                Lat: {geom.coordinates.lat.toFixed(6)}
                <br />
                Lng: {geom.coordinates.lng.toFixed(6)}
              </div>
            </Popup>
          </Marker>
        );

      case "LineString":
        return (
          <Polyline
            positions={geom.coordinates.map((c) => [c.lat, c.lng])}
            color="#3b82f6"
            weight={3}
            opacity={0.8}
          >
            <Popup>
              <div className="text-sm">
                <strong>LineString</strong>
                <br />
                {geom.coordinates.length} points
              </div>
            </Popup>
          </Polyline>
        );

      case "Polygon":
        return (
          <Polygon
            positions={geom.coordinates.map((ring) => ring.map((c) => [c.lat, c.lng] as [number, number]))}
            color="#3b82f6"
            fillColor="#3b82f6"
            fillOpacity={0.3}
            weight={2}
          >
            <Popup>
              <div className="text-sm">
                <strong>Polygon</strong>
                <br />
                {geom.coordinates.length} ring(s)
              </div>
            </Popup>
          </Polygon>
        );

      case "MultiPoint":
        return (
          <>
            {geom.coordinates.map((coord, idx) => (
              <Marker key={idx} position={[coord.lat, coord.lng]} icon={defaultIcon}>
                <Popup>
                  <div className="text-sm">
                    <strong>Point {idx + 1}</strong>
                    <br />
                    Lat: {coord.lat.toFixed(6)}
                    <br />
                    Lng: {coord.lng.toFixed(6)}
                  </div>
                </Popup>
              </Marker>
            ))}
          </>
        );

      case "MultiLineString":
        return (
          <>
            {geom.coordinates.map((line, idx) => (
              <Polyline
                key={idx}
                positions={line.map((c) => [c.lat, c.lng])}
                color="#3b82f6"
                weight={3}
                opacity={0.8}
              >
                <Popup>
                  <div className="text-sm">
                    <strong>Line {idx + 1}</strong>
                    <br />
                    {line.length} points
                  </div>
                </Popup>
              </Polyline>
            ))}
          </>
        );

      case "MultiPolygon":
        return (
          <>
            {geom.coordinates.map((polygon, idx) => (
              <Polygon
                key={idx}
                positions={polygon.map((ring) => ring.map((c) => [c.lat, c.lng] as [number, number]))}
                color="#3b82f6"
                fillColor="#3b82f6"
                fillOpacity={0.3}
                weight={2}
              >
                <Popup>
                  <div className="text-sm">
                    <strong>Polygon {idx + 1}</strong>
                    <br />
                    {polygon.length} ring(s)
                  </div>
                </Popup>
              </Polygon>
            ))}
          </>
        );

      case "GeometryCollection":
        return <>{geom.geometries.map((g, idx) => <div key={idx}>{renderGeometry(g)}</div>)}</>;

      default:
        return null;
    }
  };

  return (
    <div className="flex flex-col h-full bg-card border-l">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b bg-card/50">
        <div className="flex items-center gap-3">
          <MapPin className="h-5 w-5 text-green-600 dark:text-green-400" />
          <div>
            <h3 className="text-sm font-semibold">Geographic Data Viewer</h3>
            {columnName && (
              <div className="flex items-center gap-2 mt-1">
                <Badge variant="outline" className="text-xs font-mono">
                  {columnName}
                </Badge>
                {rowIndex !== undefined && (
                  <span className="text-xs text-muted-foreground">Row {rowIndex + 1}</span>
                )}
              </div>
            )}
          </div>
        </div>

        <div className="flex items-center gap-1">
          {onToggleFullscreen && (
            <Button variant="ghost" size="icon" onClick={onToggleFullscreen}>
              {isFullscreen ? (
                <Minimize2 className="h-4 w-4" />
              ) : (
                <Maximize2 className="h-4 w-4" />
              )}
            </Button>
          )}
          <Button variant="ghost" size="icon" onClick={onClose}>
            <X className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Geometry Info */}
      <div className="px-4 py-2 border-b bg-muted/30">
        <div className="flex items-center gap-4 text-xs">
          <div className="flex items-center gap-2">
            <span className="text-muted-foreground">Type:</span>
            <Badge variant="secondary" className="font-mono">
              {geometry.type}
            </Badge>
          </div>
          <div className="flex items-center gap-2">
            <span className="text-muted-foreground">Center:</span>
            <span className="font-mono text-foreground">
              {center.lat.toFixed(6)}, {center.lng.toFixed(6)}
            </span>
          </div>
        </div>
      </div>

      {/* Map */}
      <div className="flex-1 relative">
        <MapContainer
          center={[center.lat, center.lng]}
          zoom={13}
          className="h-full w-full"
          zoomControl={true}
        >
          <TileLayer
            attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>'
            url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
          />
          {renderGeometry(geometry)}
          <FitBounds geometry={geometry} />
        </MapContainer>
      </div>

      {/* Attribution */}
      <div className="px-4 py-2 border-t bg-muted/30">
        <p className="text-xs text-muted-foreground text-center">
          Map data © OpenStreetMap contributors
        </p>
      </div>
    </div>
  );
}

# DataSpeak Features

## Database Support

### Comprehensive Data Type Support

#### PostgreSQL
- ✅ **Numeric**: INT2, INT4, INT8, FLOAT4, FLOAT8, NUMERIC, DECIMAL (preserved as strings for precision)
- ✅ **Date/Time**: DATE, TIME, TIMESTAMP, TIMESTAMPTZ
- ✅ **Text**: VARCHAR, CHAR, TEXT
- ✅ **Boolean**: BOOL
- ✅ **UUID**: Native UUID type
- ✅ **JSON**: JSON, JSONB with native value support
- ✅ **Arrays**: All array types (displayed as strings)
- ✅ **Binary**: BYTEA (converted to hex with 0x prefix)
- ✅ **PostGIS Geometry**: POINT, LINESTRING, POLYGON, MULTIPOINT, MULTILINESTRING, MULTIPOLYGON, GEOMETRYCOLLECTION

#### MySQL/MariaDB
- ✅ **Numeric**: TINYINT, SMALLINT, MEDIUMINT, INT, BIGINT (signed & unsigned), FLOAT, DOUBLE, DECIMAL
- ✅ **Date/Time**: DATE, TIME, DATETIME, TIMESTAMP, YEAR
- ✅ **Text**: VARCHAR, CHAR, TEXT variants
- ✅ **Boolean**: BOOLEAN (TINYINT(1))
- ✅ **JSON**: Native JSON type
- ✅ **Binary**: BINARY, VARBINARY, BLOB variants (hex encoded with size limit for display)
- ✅ **Spatial**: POINT, LINESTRING, POLYGON, MULTIPOINT, MULTILINESTRING, MULTIPOLYGON, GEOMETRY
- ✅ **Special**: ENUM, SET

### Data Grid Features

#### Visual Type Indicators
- **Dates/Timestamps**: Green color (e.g., `2025-01-15 14:30:00`)
- **Numbers**: Purple color with locale formatting (e.g., `1,234,567`)
- **Booleans**: Blue badge style (e.g., `true`)
- **UUIDs**: Cyan monospace (e.g., `550e8400-e29b-41d4-a716-446655440000`)
- **Binary Data**: Orange hex strings with ellipsis for large data (e.g., `0x48656c6c6f...`)
- **JSON**: Indigo color with pretty-printing
- **NULL values**: Gray italic text
- **Geographic Data**: Green with map pin icon (clickable)

#### Editable Data Grid (NEW!)

##### Cell Editing
- Double-click any cell to edit inline
- Or click the edit icon that appears on hover
- Press Enter to save, Escape to cancel
- Visual feedback with Save/Cancel buttons

##### Row Operations
- **Delete Row**: Click trash icon (shows strikethrough with red background)
- **Add Row**: Click "Add Row" button (shows green background)
- **Restore Deleted Row**: Click restore icon on deleted rows

##### Change Tracking
- **Edited Cells**: Blue background highlight
- **Deleted Rows**: Red background + strikethrough + opacity
- **New Rows**: Green background highlight
- **Change Summary**: Badge counters showing edits/deletes/inserts

##### Commit UI
- Visual change counter in header
- "Reset" button to discard all pending changes
- "Commit" button to persist changes to database
- Color-coded badges for different change types

##### Keyboard Shortcuts
- **Double-click**: Start editing cell
- **Enter**: Save current edit
- **Escape**: Cancel current edit
- **Tab**: Navigate between cells (planned)

## Geographic Data Visualization

### Interactive Maps with Leaflet

#### Supported Geometry Types
- **POINT**: Single location marker with popup
- **LINESTRING**: Path/route visualization
- **POLYGON**: Area boundaries with fill
- **MULTIPOINT**: Multiple location markers
- **MULTILINESTRING**: Multiple paths
- **MULTIPOLYGON**: Multiple areas
- **GEOMETRYCOLLECTION**: Mixed geometry collection

#### Map Features
- **Click to View**: Click any geographic cell to open map in split pane
- **Interactive**: Pan, zoom, click markers for details
- **Auto-Fit Bounds**: Automatically centers and zooms to show all geometry
- **Fullscreen Toggle**: Expand map to full width
- **Split Pane**: Resizable layout showing both data grid and map
- **Dark Mode**: Fully themed for light and dark modes
- **OpenStreetMap**: Free, open-source map tiles

#### WKT Format Support
Automatically parses Well-Known Text format:
```sql
-- Example queries that work with map viewer
SELECT name, ST_AsText(location) FROM places;
SELECT route, ST_AsText(path) FROM trails;
SELECT region, ST_AsText(boundary) FROM areas;
```

**Note**: For PostgreSQL PostGIS, use `ST_AsText()` to convert geometry to WKT format:
```sql
-- Good: Returns WKT string
SELECT ST_AsText(geom) FROM spatial_table;

-- Binary (requires ST_AsText): Returns EWKB binary
SELECT geom FROM spatial_table;
```

### AI Assistant with Map Support

#### Location Queries
Ask the AI assistant about locations and get interactive maps:

**Example Queries**:
- "Where are our stores located?"
- "Show me the delivery routes"
- "Which regions have the most sales?"
- "Find stores within 10km of downtown"

The AI will:
1. Execute the appropriate SQL query with geographic data
2. Detect location/geometry columns in the results
3. Automatically render an interactive map showing the locations
4. Provide context and insights about the geographic data

#### Map Display in Chat
- Maps appear inline with AI responses
- Includes title and description from AI context
- 400px height with full interactivity
- Green-themed card matching geographic data styling

## Query Workspace

### Multi-Tab Interface
- **Query Tabs**: SQL editor with results
- **Table Tabs**: Browse table data with pagination
- **Chat Tab**: AI assistant for natural language queries
- Multiple tabs open simultaneously
- Easy tab switching and management

### Split Views
- **Data + Visualization**: Chart alongside data grid
- **Data + Map**: Geographic visualization alongside results
- **Resizable Panes**: Drag to adjust layout
- **Grid Only**: Full-width data view

### Results Display
- Fast pagination (10, 20, 50, 100, 200 rows per page)
- Sticky headers for scrolling
- Execution time and row count display
- Error messages with helpful context

## Planned Features

### Editable Grid Backend (In Progress)
- [ ] Rust backend endpoints for UPDATE/DELETE/INSERT
- [ ] Transaction support for atomic commits
- [ ] Optimistic UI updates
- [ ] Error handling and rollback

### UI Store Integration
- [ ] Move map/geography state to UI store
- [ ] Centralized state management for modals and panels
- [ ] Persistent UI preferences

### Advanced Editing
- [ ] Tab navigation between cells
- [ ] Bulk edit operations
- [ ] Import/export with change tracking
- [ ] Undo/redo support

### AI Enhancements
- [ ] Automatic geometry detection in queries
- [ ] Smart map suggestions for location data
- [ ] Route planning and distance calculations
- [ ] Spatial analysis insights

## Technical Implementation

### Frontend
- **React 18** with TypeScript
- **TanStack Table** for data grids
- **Leaflet** & **React-Leaflet** for maps
- **Tailwind CSS** for styling
- **shadcn/ui** component library
- **Zustand** for state management

### Backend
- **Rust** with Tauri v2
- **SQLx** for database connectivity
- **Async/await** with Tokio runtime
- **Strong typing** throughout
- **Error handling** with Result types

### Database Connectivity
- PostgreSQL (including PostGIS)
- MySQL
- MariaDB
- Connection pooling and management
- Prepared statements for security

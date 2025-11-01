# Editable DataGrid - Usage Guide

## How to Edit, Delete, and Add Rows

### 1. Edit a Cell

There are **two ways** to edit a cell:

#### Method A: Double-Click
1. **Double-click** any cell in the data grid
2. An input field will appear with the current value selected
3. Type your new value
4. Press **Enter** to save or **Escape** to cancel
5. The cell will turn **blue** to show it's been edited

#### Method B: Click Edit Button
1. **Hover** over any cell
2. A small **pencil icon** (Edit3) will appear on the right side
3. **Click** the pencil icon
4. An input field will appear
5. Type your new value and press **Enter** to save

**Visual Feedback:**
- Edited cells have a **blue background**
- A blue badge appears in the header showing the count

### 2. Delete a Row

1. Look at the **"Actions"** column on the right side of the table
2. Each row has a **trash icon** 🗑️
3. **Click** the trash icon to delete that row
4. The row will show:
   - **Strikethrough text**
   - **Red background**
   - Reduced opacity
   - The trash icon changes to a **restore icon** ↻

**To Restore a Deleted Row:**
- Click the **restore icon** (↻) in the Actions column
- The row returns to normal

**Visual Feedback:**
- Deleted rows have **red background** and strikethrough
- A red badge in the header shows the delete count

### 3. Add a New Row

1. Look at the top-right of the data grid header
2. Click the **"Add Row"** button (with + icon)
3. A new row appears at the bottom with NULL values
4. The row has a **green background**
5. **Edit the cells** in the new row using Method 1 or 2 above

**Visual Feedback:**
- New rows have a **green background**
- A green badge in the header shows the insert count

### 4. Commit or Reset Changes

After making changes, you have two options:

#### Commit Changes (Save to Database)
1. Click the **"Commit"** button in the header
2. The button shows the total number of changes
3. Example: "Commit (5)" means 5 total changes
4. **Note:** Backend persistence is not yet implemented, but the UI is ready

#### Reset Changes (Discard All)
1. Click the **"Reset"** button in the header
2. All pending changes will be discarded
3. The grid returns to the original data state
4. All color highlights disappear

## Change Tracking Summary

The grid tracks three types of changes:

| Change Type | Visual Indicator | Badge Color | Description |
|------------|------------------|-------------|-------------|
| **Edit** | Blue background on cell | Blue | Cell value modified |
| **Delete** | Red background + strikethrough | Red | Row marked for deletion |
| **Insert** | Green background on row | Green | New row added |

## Example Workflow

### Scenario: Update employee data

1. **Edit a salary:**
   - Double-click the salary cell for "John Doe"
   - Type "75000"
   - Press Enter
   - Cell turns blue

2. **Delete an old record:**
   - Click trash icon for the employee who left
   - Row turns red with strikethrough

3. **Add a new employee:**
   - Click "Add Row"
   - Edit name cell: "Jane Smith"
   - Edit salary cell: "65000"
   - Edit department cell: "Engineering"
   - All cells in green row

4. **Review changes:**
   - Header shows: "3 edited, 1 deleted, 1 added"
   - Blue cells, red row, green row all visible

5. **Commit or Reset:**
   - Click "Commit (5)" to save
   - Or click "Reset" to undo everything

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| **Double-click** | Start editing cell |
| **Enter** | Save current edit |
| **Escape** | Cancel current edit |

## Important Notes

1. **NULL Values:** Empty cells or cleared values become NULL
2. **Data Types:** The grid preserves data types (numbers, dates, booleans)
3. **Geographic Data:** Cells with WKT geometry show map icon and remain clickable
4. **Read-only when deleted:** You cannot edit cells in deleted rows
5. **Validation:** Type checking happens when you edit (planned)
6. **Backend:** Commit functionality needs backend endpoints (in development)

## Color Reference

- 🔵 **Blue** = Edited cell
- 🔴 **Red** = Deleted row
- 🟢 **Green** = New row
- ⚫ **Gray** = NULL value
- 🟣 **Purple** = Number
- 🟢 **Green text** = Date/Time/Geographic
- 🔵 **Blue badge** = Boolean
- 🟠 **Orange** = Binary data
- 🟡 **Cyan** = UUID

## Tips

- **Hover to discover:** Hover over cells to see the edit button
- **Batch operations:** Make all your changes, then commit once
- **Review before commit:** Check the change badges before clicking Commit
- **Undo is easy:** Just click Reset to discard everything
- **Geographic cells:** You can still click map pins to view location while editing

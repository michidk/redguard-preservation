# MPOB Scripted-Object Spawning Runtime Path Analysis

**Analysis Date:** 2026-05-02  
**Scope:** redguard-unity MPOB loading, parsing, instantiation, and positioning  
**Focus:** Identify exact files/functions and compare against RGPL placement path

---

## Executive Summary

The MPOB (scripted object) spawning path in redguard-unity follows this flow:

1. **Load** → `FFIModelLoader.LoadArea()` calls `LoadRgmSections()`
2. **Parse** → `FFIModelLoader.ParseMPOB()` reads MPOB binary section
3. **Instantiate** → `RGScriptedObject.Instanciate()` creates GameObject + component
4. **Position** → Position/rotation applied in `Instanciate()` method

**Key Finding:** The MPOB path uses **consistent scaling** (`RGM_MPOB_SCALE = 1/5120.0f`) throughout, with **axis conversions** that differ from RGPL. No double-scaling detected.

---

## File Map

| File | Role |
|------|------|
| `Assets/Scripts/FFI/FFIModelLoader.cs` | MPOB loading orchestrator |
| `Assets/Scripts/RGFileImport/RGMData/RGScriptedObject.cs` | MPOB instantiation + positioning |
| `Assets/Scripts/FFI/RgmdDeserializer.cs` | Mesh deserialization (used by both paths) |
| `Assets/Scripts/FFI/RgplDeserializer.cs` | RGPL placement deserialization (comparison) |

---

## Detailed Flow

### 1. MPOB Loading Entry Point

**File:** `FFIModelLoader.cs:193–238`  
**Function:** `LoadArea(string areaName, string paletteName, string wldName)`

```csharp
public static List<GameObject> LoadArea(string areaName, string paletteName, string wldName)
{
    // ... setup ...
    string rgmPath = Path.Combine(mapsFolder, areaName + ".RGM");
    if (File.Exists(rgmPath))
    {
        LoadRgmSections(paletteName, meshDict, objects);  // ← MPOB parsing happens here
    }
    // ... RGPL placement and terrain ...
}
```

**Key:** MPOB is loaded **before** RGPL placements (line 224 vs 229).

---

### 2. RGM Section Population

**File:** `FFIModelLoader.cs:699–728`  
**Function:** `LoadRgmSections()`

```csharp
private static void LoadRgmSections(
    string paletteName,
    Dictionary<string, (Mesh mesh, ...)> meshDict,
    List<GameObject> objects)
{
    CurrentRgmData = new RGRGMFile();
    
    PopulateRawSections();                    // Load all RGM sections into memory
    ParseRAHD(GetSection("RAHD"));            // Parse RAHD (object metadata)
    ParseMPOB(GetSection("MPOB"), ...);       // ← Parse MPOB (scripted objects)
    
    // Initialize animation and script stores
    RGRGMAnimStore.ReadAnim(CurrentRgmData);
    RGRGMScriptStore.ReadScript(CurrentRgmData);
}
```

**Key:** MPOB section is fetched via `GetSection("MPOB")` (line 708).

---

### 3. MPOB Binary Parsing

**File:** `FFIModelLoader.cs:856–929`  
**Function:** `ParseMPOB(byte[] mpobBytes, string paletteName, ...)`

#### 3.1 MPOB Item Structure Parsing

```csharp
var reader = new RGFileImport.MemoryReader(mpobBytes);
uint numItems = (uint)reader.ReadInt32();

for (int i = 0; i < (int)numItems; i++)
{
    var mpob = new RGRGMFile.RGMMPOBItem();
    
    mpob.id = (uint)reader.ReadInt32();                     // 4 bytes
    mpob.type = (RGRGMFile.ObjectType)reader.ReadByte();    // 1 byte
    mpob.isActive = reader.ReadByte();                      // 1 byte
    
    char[] scriptChars = reader.ReadChars(9);               // 9 bytes
    mpob.scriptName = new string(scriptChars).Split('\0')[0];
    
    char[] modelChars = reader.ReadChars(9);                // 9 bytes
    string rawModel = new string(modelChars).Split('\0')[0];
    mpob.modelName = NormalizeModelName(rawModel);
    
    mpob.isStatic = reader.ReadByte();                      // 1 byte
    mpob.unknown1 = reader.ReadInt16();                     // 2 bytes
    
    // ← POSITION PARSING (24-bit + padding)
    mpob.posX = reader.ReadByte() | (reader.ReadByte() << 8) | (reader.ReadByte() << 16);
    reader.ReadByte();  // padding
    mpob.posY = reader.ReadByte() | (reader.ReadByte() << 8) | (reader.ReadByte() << 16);
    reader.ReadByte();  // padding
    mpob.posZ = reader.ReadByte() | (reader.ReadByte() << 8) | (reader.ReadByte() << 16);
    // (no padding after posZ in current code)
    
    // ← ROTATION PARSING (32-bit angles)
    mpob.anglex = reader.ReadInt32();                       // 4 bytes
    mpob.angley = reader.ReadInt32();                       // 4 bytes
    mpob.anglez = reader.ReadInt32();                       // 4 bytes
    
    // ← TEXTURE/LIGHT DATA
    ushort textureData = reader.ReadUInt16();               // 2 bytes
    mpob.textureId = (byte)(textureData >> 7);
    mpob.imageId = (byte)(textureData & 0x7F);
    mpob.intensity = reader.ReadInt16();                    // 2 bytes
    mpob.radius = reader.ReadInt16();                       // 2 bytes
    mpob.modelId = reader.ReadInt16();                      // 2 bytes
    mpob.worldId = reader.ReadInt16();                      // 2 bytes
    mpob.red = reader.ReadInt16();                          // 2 bytes
    mpob.green = reader.ReadInt16();                        // 2 bytes
    mpob.blue = reader.ReadInt16();                         // 2 bytes
}
```

**Observations:**
- Position is **24-bit unsigned** (0–16777215 / 0xFFFFFF)
- Rotation angles are **32-bit signed** integers
- Texture data is **bit-packed** (7 bits for textureId, 7 bits for imageId)

#### 3.2 GameObject Creation & Instantiation

```csharp
try
{
    string objectName = "B_" + i.ToString("D3") + "_" + (mpob.scriptName ?? "OBJ");
    GameObject go = new GameObject(objectName);
    RGScriptedObject scripted = go.AddComponent<RGScriptedObject>();
    scripted.Instanciate(mpob, CurrentRgmData, paletteName);  // ← Position/rotation applied here
    ScriptedObjects[mpob.id] = scripted;
    objects.Add(go);
}
catch (Exception ex)
{
    Debug.LogWarning("[FFI] Failed to spawn scripted object " + mpob.scriptName + ": " + ex.Message);
}
```

**Key:** The GameObject is created **empty**, then `RGScriptedObject.Instanciate()` applies position, rotation, and mesh.

---

### 4. MPOB Instantiation & Positioning

**File:** `RGScriptedObject.cs:243–354`  
**Function:** `Instanciate(RGMMPOBItem MPOB, RGRGMFile filergm, string name_col)`

#### 4.1 Position Calculation

```csharp
const float RGM_MPOB_SCALE = 1/5120.0f;  // Line 36

Vector3 position = Vector3.zero;
position.x = (float)(MPOB.posX) * RGM_MPOB_SCALE;           // Line 253
position.y = -(float)(MPOB.posY) * RGM_MPOB_SCALE;          // Line 254 (negated)
position.z = -(float)(0xFFFFFF - MPOB.posZ) * RGM_MPOB_SCALE;  // Line 255 (inverted + negated)

transform.position = position;  // Line 257
```

**Axis Conversion:**
| Axis | Formula | Notes |
|------|---------|-------|
| X | `posX * scale` | Direct |
| Y | `-posY * scale` | Negated (Y-flip) |
| Z | `-(0xFFFFFF - posZ) * scale` | Inverted (0xFFFFFF - posZ) then negated |

**Equivalent Z formula:** `-(0xFFFFFF - posZ) = posZ - 0xFFFFFF`

#### 4.2 Rotation Calculation

```csharp
Vector3 rotation = EulerFromMpobData(MPOB);  // Line 256
transform.Rotate(rotation);                   // Line 258

private static Vector3 EulerFromMpobData(RGFileImport.RGRGMFile.RGMMPOBItem item)
{
    const float da2dg = 180.0f / 1024.0f;  // Angle-to-degree conversion
    Vector3 eulers = new Vector3(item.anglex % 2048, item.angley % 2048, item.anglez % 2048);
    eulers *= da2dg;
    return Vector3.Scale(eulers, new Vector3(1f, 1f, 1f));  // No axis flip
}
```

**Rotation Details:**
- Angles are **modulo 2048** (wrapping)
- Conversion: `angle_degrees = angle_raw * (180 / 1024)`
- **No axis flipping** applied to rotation (unlike position)

#### 4.3 RALC Location Offsets

```csharp
locations = new List<Vector3>();
locations.Add(position);

int RALC_offset = RAHDData.RALCOffset / 12;  // RALC items are 12 bytes each
for (int i = 0; i < RAHDData.RALCCount; i++)
{
    if (RALC_offset + i >= filergm.RALC.items.Count)
        break;
    
    RGRGMFile.RGMRALCItem RALCData = filergm.RALC.items[RALC_offset + i];
    Vector3 loc = position;
    loc.x += (float)(RALCData.offsetX) * RGM_MPOB_SCALE;
    loc.y += -(float)(RALCData.offsetY) * RGM_MPOB_SCALE;
    loc.z += -(float)(RALCData.offsetZ) * RGM_MPOB_SCALE;
    locations.Add(loc);
}
```

**Key:** RALC offsets use **same axis conversion** as base position (Y and Z negated).

#### 4.4 Type-Specific Instantiation

```csharp
switch (MPOB.type)
{
    case RGFileImport.RGRGMFile.ObjectType.object_3d:
        Instanciate3DObject(MPOB, filergm, name_col);
        break;
    case RGFileImport.RGRGMFile.ObjectType.object_lightobject:
        InstanciateLightObject(MPOB, filergm, name_col);
        break;
    case RGFileImport.RGRGMFile.ObjectType.object_light:
        InstanciateLight(MPOB, filergm);
        break;
    case RGFileImport.RGRGMFile.ObjectType.object_flat:
        InstanciateFlat(MPOB, filergm, name_col);
        break;
    case RGFileImport.RGRGMFile.ObjectType.object_audio:
    default:
        Debug.Log($"unhandled type: {MPOB.type} for object {MPOB.scriptName} with model {MPOB.modelName}");
        break;
}
```

**Mesh Loading (3D objects):**

```csharp
public void Instanciate3DObject(RGRGMFile.RGMMPOBItem MPOB, RGRGMFile filergm, string name_col)
{
    skinnedMeshRenderer = gameObject.AddComponent<SkinnedMeshRenderer>();
    
    try
    {
        animations = new AnimData(MPOB.scriptName);
    }
    catch
    {
        animations = null;
    }
    
    if (animations != null && animations.animationData.RAANItems.Count > 0)
    {
        // Animated object: load multiple mesh frames
        // ...
    }
    else
    {
        // Static object: load single mesh
        string modelname = !string.IsNullOrEmpty(MPOB.modelName) 
            ? MPOB.modelName.Split('.')[0] 
            : MPOB.scriptName;
        
        if (FFIModelLoader.TryGetMeshData(modelname, name_col, out Mesh mesh, out List<Material> materials, out _))
        {
            skinnedMeshRenderer.sharedMesh = mesh;
            skinnedMeshRenderer.SetMaterials(materials);
        }
    }
}
```

**Key:** Mesh is loaded via `FFIModelLoader.TryGetMeshData()` — **no scaling applied at this stage**.

---

## Comparison: MPOB vs RGPL Placement Path

### RGPL Path (for reference)

**File:** `FFIModelLoader.cs:240–331`  
**Function:** `PlaceRgplObjects()`

```csharp
IntPtr resultPtr = RgpreBindings.GetWorldPlacements(CurrentWorldHandle);
var rgpl = RgplDeserializer.Deserialize(resultPtr);

foreach (var placement in rgpl.placements)
{
    // ... mesh loading ...
    GameObject obj = CreateGameObject(objectName, mesh, materials, frameCount);
    ApplyMatrix(obj.transform, placement.transform);  // ← Matrix applied here
    objects.Add(obj);
}

private static void ApplyMatrix(Transform transform, Matrix4x4 matrix)
{
    transform.position = new Vector3(matrix.m03, matrix.m13, matrix.m23);
    
    Vector3 scale = new Vector3(
        new Vector3(matrix.m00, matrix.m10, matrix.m20).magnitude,
        new Vector3(matrix.m01, matrix.m11, matrix.m21).magnitude,
        new Vector3(matrix.m02, matrix.m12, matrix.m22).magnitude
    );
    transform.localScale = scale;
    
    if (scale.x > 0f && scale.y > 0f && scale.z > 0f)
    {
        Matrix4x4 rotMatrix = Matrix4x4.identity;
        rotMatrix.SetColumn(0, matrix.GetColumn(0) / scale.x);
        rotMatrix.SetColumn(1, matrix.GetColumn(1) / scale.y);
        rotMatrix.SetColumn(2, matrix.GetColumn(2) / scale.z);
        transform.rotation = rotMatrix.rotation;
    }
}
```

### Key Differences

| Aspect | MPOB | RGPL |
|--------|------|------|
| **Position Source** | 24-bit unsigned integers (posX, posY, posZ) | 4×4 transformation matrix |
| **Scaling** | Fixed `RGM_MPOB_SCALE = 1/5120.0f` | Extracted from matrix magnitude |
| **Axis Conversion** | Y negated, Z inverted+negated | Matrix columns (no explicit conversion) |
| **Rotation** | Euler angles (32-bit signed) | Extracted from rotation matrix |
| **Mesh Scaling** | No per-object scaling | Per-object scale from matrix |

---

## Potential Issues & Observations

### 1. **No Double-Scaling Detected**

✅ **MPOB path:**
- Position: `posX * (1/5120)` — single scale
- Mesh: No additional scaling applied
- Result: **Single scaling pass**

✅ **RGPL path:**
- Position: From matrix translation
- Mesh: Scale extracted from matrix magnitude
- Result: **Single scaling pass**

**Conclusion:** Both paths apply scaling once. No double-scaling bug.

---

### 2. **Axis Conversion Consistency**

**MPOB:**
```
X: posX * scale
Y: -posY * scale
Z: -(0xFFFFFF - posZ) * scale
```

**RGPL:**
- Uses matrix columns directly (no explicit conversion visible in C# code)
- Actual conversion happens in native Rust code (`RgpreBindings.GetWorldPlacements()`)

**Observation:** MPOB and RGPL may use **different coordinate systems**. If RGPL matrix is already in Unity space, but MPOB raw values need conversion, this could cause misalignment.

---

### 3. **Rotation Handling**

**MPOB:**
- Angles: 32-bit signed integers
- Conversion: `angle_degrees = angle_raw * (180 / 1024)`
- Application: `transform.Rotate(eulerAngles)` — **relative rotation**

**RGPL:**
- Rotation: Extracted from matrix columns
- Application: `transform.rotation = quaternion` — **absolute rotation**

**Potential Issue:** `transform.Rotate()` applies **relative** rotation, while matrix extraction gives **absolute** rotation. If the GameObject already has a default rotation, this could cause double-rotation.

---

### 4. **Mesh Collider Assignment**

```csharp
if (skinnedMeshRenderer != null)
{
    if (skinnedMeshRenderer.sharedMesh != null)
    {
        gameObject.AddComponent<MeshCollider>();
        gameObject.GetComponent<MeshCollider>().sharedMesh = skinnedMeshRenderer.sharedMesh;
    }
}
```

**Observation:** MeshCollider is added **after** position/rotation are set. This is correct (collider inherits transform).

---

### 5. **Model Name Normalization**

```csharp
mpob.modelName = NormalizeModelName(rawModel);
```

**File:** `FFIPathUtils.NormalizeModelName()`

**Observation:** Model names are normalized (likely case-insensitive, extension handling). This should match RGPL path, but worth verifying if model lookup is failing.

---

### 6. **RAHD Metadata Dependency**

```csharp
if (!filergm.RAHD.dict.TryGetValue(scriptName, out RAHDData))
{
    RAHDData = new RGFileImport.RGRGMFile.RGMRAHDItem();
}
```

**Observation:** If RAHD entry is missing, a **default empty item** is created. This could cause:
- Missing animation data (RAAN)
- Missing location offsets (RALC)
- Missing attributes (RAAT)

**Potential Bug:** If RAHD is corrupted or incomplete, MPOB objects may spawn with wrong data.

---

## Suspected Runtime Issues

### Issue 1: Rotation Applied as Relative Instead of Absolute

**Location:** `RGScriptedObject.cs:258`

```csharp
transform.Rotate(rotation);  // ← Relative rotation
```

**Problem:** If the GameObject's initial rotation is not identity, `Rotate()` will compound the rotation.

**Fix:** Use `transform.rotation = Quaternion.Euler(rotation);` instead.

---

### Issue 2: Z-Axis Inversion Formula Ambiguity

**Location:** `RGScriptedObject.cs:255`

```csharp
position.z = -(float)(0xFFFFFF - MPOB.posZ) * RGM_MPOB_SCALE;
```

**Problem:** This is equivalent to `(MPOB.posZ - 0xFFFFFF) * RGM_MPOB_SCALE`. If MPOB.posZ is unsigned 24-bit, the subtraction `0xFFFFFF - MPOB.posZ` is correct. But the double negation is confusing.

**Clarification:** 
- `0xFFFFFF - posZ` inverts the Z coordinate (0 ↔ 16777215)
- The outer negation flips the sign
- Net effect: Z-axis flip + inversion

**Verification needed:** Compare with RGPL matrix Z column to ensure this matches.

---

### Issue 3: RALC Offset Calculation

**Location:** `RGScriptedObject.cs:264`

```csharp
int RALC_offset = RAHDData.RALCOffset / 12;
```

**Problem:** RALC items are 12 bytes each (3 × int32). The offset is divided by 12 to get the item index. If `RAHDData.RALCOffset` is not a multiple of 12, this will truncate.

**Observation:** This assumes RALC offsets are always byte-aligned to 12-byte boundaries. If not, this could skip or misalign RALC data.

---

### Issue 4: Missing Mesh Collider Convex Flag

**Location:** `RGScriptedObject.cs:331–332`

```csharp
gameObject.AddComponent<MeshCollider>();
gameObject.GetComponent<MeshCollider>().sharedMesh = skinnedMeshRenderer.sharedMesh;
```

**Problem:** MeshCollider is created without setting `convex = true`. For complex meshes, this can cause physics issues.

**Observation:** RGPL path also doesn't set convex, so this is consistent but may be a shared issue.

---

## Summary Table: MPOB Runtime Path

| Stage | File | Function | Key Action |
|-------|------|----------|------------|
| **Load** | FFIModelLoader.cs | LoadArea() | Orchestrates RGM section loading |
| **Fetch** | FFIModelLoader.cs | LoadRgmSections() | Retrieves MPOB binary section |
| **Parse** | FFIModelLoader.cs | ParseMPOB() | Reads MPOB items from binary |
| **Create** | FFIModelLoader.cs | ParseMPOB() | Creates GameObject + RGScriptedObject |
| **Position** | RGScriptedObject.cs | Instanciate() | Applies position (24-bit → float) |
| **Rotate** | RGScriptedObject.cs | Instanciate() | Applies rotation (Euler angles) |
| **Mesh** | RGScriptedObject.cs | Instanciate3DObject() | Loads mesh via FFIModelLoader |
| **Collider** | RGScriptedObject.cs | Instanciate() | Adds MeshCollider |

---

## Recommendations for Investigation

1. **Verify Z-axis formula** against RGPL matrix Z column
2. **Check rotation application** — compare `transform.Rotate()` vs `transform.rotation = Quaternion.Euler()`
3. **Inspect RAHD parsing** — ensure RAHD entries match MPOB count
4. **Test RALC offset alignment** — verify offsets are multiples of 12
5. **Compare mesh positions** — place same object via MPOB and RGPL, measure offset
6. **Check model name normalization** — ensure MPOB model names match file system

---

## External References

- **MPOB Format:** `docs/formats/RGM.md` (if available)
- **RGPL Format:** `docs/formats/RGPL.md` (if available)
- **RGUnity RGScriptedObject:** [RGUnity/redguard-unity `RGScriptedObject.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/RGFileImport/RGMData/RGScriptedObject.cs)
- **RGUnity FFIModelLoader:** [RGUnity/redguard-unity `FFIModelLoader.cs`](https://github.com/RGUnity/redguard-unity/blob/master/Assets/Scripts/FFI/FFIModelLoader.cs)

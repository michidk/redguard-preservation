# Unity GLB Export Transform Analysis

## Summary

The Unity redguard-unity project uses **glTFast** (com.unity.cloud.gltfast) for GLB export. The export pipeline **preserves mesh-local geometry as-is** (with coordinate system flips) and **stores node transforms separately** in the glTF node hierarchy. **Transforms are NOT baked into vertex positions.**

---

## Export Pipeline Overview

### Entry Point
- **File**: `Assets/Scripts/ModelViewer/Export/GLTFExporter.cs`
- **Method**: `ExportGLTF(GameObject obj, string objectName)`
- Creates a `GameObjectExport` instance with `ExportSettings` and calls `export.AddScene(objectsToExport)`

### Core Export Classes

#### 1. GameObjectExport (glTFast)
**File**: `Library/PackageCache/com.unity.cloud.gltfast@c1ac217282ac/Runtime/Scripts/Export/GameObjectExport.cs`

**Key Method**: `AddScene(ICollection<GameObject> gameObjects, float4x4 origin, string name)` (lines 69-109)

**Transform Handling**:
```csharp
// Lines 207-219: Transform extraction
if (sceneOrigin.HasValue)
{
    // root level node - calculate transform based on scene origin
    var trans = math.mul(sceneOrigin.Value, transform.localToWorldMatrix);
    trans.Decompose(out translation, out rotation, out scale);
}
else
{
    // nested node - use local transform
    translation = transform.localPosition;
    rotation = transform.localRotation;
    scale = transform.localScale;
}
```

**Critical Finding**: 
- **Root nodes** use `localToWorldMatrix` (world space) multiplied by scene origin
- **Child nodes** use `localPosition`, `localRotation`, `localScale` (local space)
- These transforms are stored as **node transforms in glTF**, NOT baked into mesh vertices

#### 2. GltfWriter (glTFast)
**File**: `Library/PackageCache/com.unity.cloud.gltfast@c1ac217282ac/Runtime/Scripts/Export/GltfWriter.cs`

**Key Methods**:
- `AddNode()` (lines 141-156): Creates glTF node with translation/rotation/scale
- `AddMeshToNode()` (lines 173-232): Attaches mesh to node
- `AddMesh()` (lines 2187-2236): Registers mesh for export
- `BakeMesh()` (lines 1044-1354): **Processes vertex data**

---

## Mesh Vertex Data Processing

### BakeMesh Method (lines 1044-1354)

**Key Steps**:

1. **Vertex Attribute Extraction** (lines 1053-1157)
   - Reads mesh vertex attributes from Unity mesh
   - Creates accessors for each attribute (Position, Normal, Tangent, TexCoord, etc.)
   - **No transform applied at this stage**

2. **Vertex Data Retrieval** (lines 1236-1248)
   ```csharp
   for (var stream = 0; stream < streamCount; stream++)
   {
       inputStreams[stream] = await meshData.GetVertexData(stream);
       outputStreams[stream] = new NativeArray<byte>(
           outputStrides[stream] * vertexCount, 
           Allocator.Persistent
       );
   }
   ```
   - Gets raw vertex data from Unity mesh (local space)
   - Creates output buffer for converted data

3. **Attribute Conversion** (lines 1250-1329)
   - Calls specialized conversion jobs for each attribute type
   - **Position and Normal attributes undergo coordinate system conversion**
   - Other attributes (TexCoord, Color, etc.) are copied as-is

### Coordinate System Conversion

#### ConvertPositionFloatJob (ExportJobs.cs, lines 110-133)
```csharp
public unsafe struct ConvertPositionFloatJob : IJobParallelFor
{
    public void Execute(int i)
    {
        var inPtr = (float3*)(input + i * inputByteStride);
        var outPtr = (float3*)(output + i * outputByteStride);

        var tmp = *inPtr;
        tmp.x *= -1;  // ← ONLY X-AXIS FLIP
        *outPtr = tmp;
    }
}
```

**Critical Finding**: 
- **Only X-coordinate is negated** (`tmp.x *= -1`)
- Y and Z coordinates are **preserved as-is**
- This is a **coordinate system handedness conversion** (Unity left-handed → glTF right-handed)
- **No scaling, rotation, or translation is applied**

#### ConvertPositionHalfJob (ExportJobs.cs, lines 136-159)
- Same logic for half-precision (float16) positions
- Converts to float32 during export

#### ConvertTangentFloatJob (ExportJobs.cs, lines 162-185)
```csharp
public void Execute(int i)
{
    var inPtr = (float4*)(input + i * inputByteStride);
    var outPtr = (float4*)(output + i * outputByteStride);

    var tmp = *inPtr;
    tmp.z *= -1;  // ← ONLY Z-AXIS FLIP
    *outPtr = tmp;
}
```

**Critical Finding**:
- **Only Z-component is negated** (tangent space adjustment)
- W component (handedness) is **preserved as-is**

#### ConvertTangentHalfJob (ExportJobs.cs, lines 188-200)
- Same logic for half-precision tangents

#### ConvertTexCoordAttribute (lines 1965-1986)
- **No transformation applied** (see ConvertTexCoordAttributeJob)
- UV coordinates copied directly

#### ConvertGenericAttribute (lines 1988-2008)
- **No transformation applied**
- All other attributes copied directly

### Bind Poses (Skinning)

**ConvertMatrixJob** (ExportJobs.cs, lines 289-304)
```csharp
public void Execute(int i)
{
    var tmp = matrices[i];
    tmp.c0.y *= -1;  // Column 0, Y component
    tmp.c0.z *= -1;  // Column 0, Z component
    tmp.c1.x *= -1;  // Column 1, X component
    tmp.c2.x *= -1;  // Column 2, X component
    tmp.c3.x *= -1;  // Column 3, X component (translation)
    matrices[i] = tmp;
}
```

**Critical Finding**:
- Applied to **bind pose matrices** (inverse bind matrices for skinning)
- Transforms the matrix to match the coordinate system flip
- Called from `WriteBindPosesToBuffer()` (GltfWriter.cs:1535-1556)
- **Only used for skinned meshes** (SkinnedMeshRenderer)

---

## Index Conversion

### ConvertIndicesFlippedJobUInt16/UInt32 (ExportJobs.cs, lines 20-59)
```csharp
public void Execute(int i)
{
    result[i * 3 + 0] = input[i * 3 + 0] + baseVertexOffset;
    result[i * 3 + 1] = input[i * 3 + 2] + baseVertexOffset;  // ← FLIP
    result[i * 3 + 2] = input[i * 3 + 1] + baseVertexOffset;  // ← FLIP
}
```

**Critical Finding**:
- **Triangle winding order is flipped** (indices 1 and 2 swapped)
- This compensates for the X-axis flip in positions
- Ensures correct face normals after coordinate system conversion

---

## Transform Baking Summary

| Component | Baked into Vertices? | Stored Where | Details |
|-----------|----------------------|--------------|---------|
| **Node Position** | ❌ No | glTF Node.translation | Stored as separate node transform |
| **Node Rotation** | ❌ No | glTF Node.rotation | Stored as separate node transform |
| **Node Scale** | ❌ No | glTF Node.scale | Stored as separate node transform |
| **Mesh-Local Positions** | ⚠️ Partial | Vertex Buffer | Only X-axis flipped (handedness) |
| **Mesh-Local Normals** | ⚠️ Partial | Vertex Buffer | Only X-axis flipped (handedness) |
| **Mesh-Local Tangents** | ⚠️ Partial | Vertex Buffer | Only Z-axis flipped (tangent space) |
| **Mesh-Local TexCoords** | ✅ As-is | Vertex Buffer | No transformation |
| **Triangle Winding** | ✅ Flipped | Index Buffer | Flipped to match position flip |
| **Bind Poses (Skinning)** | ⚠️ Partial | Accessor Buffer | Matrix columns flipped to match coordinate system |

---

## Implications for Rust GLB Comparison

### What Should Match
1. **Node transforms** (translation, rotation, scale) should match between Unity export and Rust parser
2. **Mesh-local geometry** should match after accounting for:
   - X-axis flip in positions
   - X-axis flip in normals
   - Z-axis flip in tangents
   - Triangle winding flip

### What Could Differ
1. **Absolute vertex positions** if parent transforms differ
2. **Mesh bounds** (min/max) if calculated differently
3. **Skinning data** (blend weights/indices) if not properly converted
4. **Material assignments** if material indices don't match

### Key Verification Points
1. Check if Rust parser applies the same X-axis flip to positions
2. Verify triangle winding is handled correctly
3. Ensure node transform hierarchy is preserved
4. Confirm mesh-local coordinates are NOT transformed by parent node transforms

---

## Files to Reference

| File | Purpose | Key Lines |
|------|---------|-----------|
| `GLTFExporter.cs` | Entry point | 10-54 |
| `GameObjectExport.cs` | Scene/node hierarchy | 69-109, 156-243 |
| `GltfWriter.cs` | Mesh export | 1044-1354, 1859-1910 |
| `ExportJobs.cs` | Vertex conversion | 110-159, 162-185 |

---

## Conclusion

**The Unity GLB exporter does NOT bake transforms into mesh vertices.** Instead:

1. **Node transforms** are preserved in the glTF node hierarchy
2. **Mesh-local geometry** is stored with minimal transformation:
   - X-axis flip (handedness conversion)
   - Triangle winding flip (to match position flip)
   - No scaling, rotation, or translation applied
3. **Mesh bounds** are calculated from original mesh bounds with X-axis flip applied

This means exported GLB files maintain the original mesh geometry in local space, with transforms applied at the node level during rendering. Any discrepancies in the Rust parser likely stem from:
- Incorrect handling of the X-axis flip
- Incorrect triangle winding
- Incorrect node transform application
- Mesh bounds calculation differences

# Unity GLB Export - Code Snippets Reference

## Transform Extraction

### Root Node Transform (GameObjectExport.cs:207-211)
```csharp
if (sceneOrigin.HasValue)
{
    // root level node - calculate transform based on scene origin
    var trans = math.mul(sceneOrigin.Value, transform.localToWorldMatrix);
    trans.Decompose(out translation, out rotation, out scale);
}
```

### Child Node Transform (GameObjectExport.cs:215-218)
```csharp
else
{
    // nested node - use local transform
    translation = transform.localPosition;
    rotation = transform.localRotation;
    scale = transform.localScale;
}
```

---

## Vertex Conversion Jobs

### Position Conversion - Float32 (ExportJobs.cs:110-133)
```csharp
[BurstCompile]
public unsafe struct ConvertPositionFloatJob : IJobParallelFor
{
    public uint inputByteStride;
    public uint outputByteStride;

    [ReadOnly]
    [NativeDisableUnsafePtrRestriction]
    public byte* input;

    [WriteOnly]
    [NativeDisableUnsafePtrRestriction]
    public byte* output;

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

### Position Conversion - Float16 (ExportJobs.cs:136-159)
```csharp
[BurstCompile]
public unsafe struct ConvertPositionHalfJob : IJobParallelFor
{
    public uint inputByteStride;
    public uint outputByteStride;

    [ReadOnly]
    [NativeDisableUnsafePtrRestriction]
    public byte* input;

    [WriteOnly]
    [NativeDisableUnsafePtrRestriction]
    public byte* output;

    public void Execute(int i)
    {
        var inPtr = (half3*)(input + i * inputByteStride);
        var outPtr = (float3*)(output + i * outputByteStride);

        var tmp = (float3)(*inPtr);
        tmp.x *= -1;  // ← ONLY X-AXIS FLIP
        *outPtr = tmp;
    }
}
```

### Tangent Conversion - Float32 (ExportJobs.cs:162-185)
```csharp
[BurstCompile]
public unsafe struct ConvertTangentFloatJob : IJobParallelFor
{
    public uint inputByteStride;
    public uint outputByteStride;

    [ReadOnly]
    [NativeDisableUnsafePtrRestriction]
    public byte* input;

    [WriteOnly]
    [NativeDisableUnsafePtrRestriction]
    public byte* output;

    public void Execute(int i)
    {
        var inPtr = (float4*)(input + i * inputByteStride);
        var outPtr = (float4*)(output + i * outputByteStride);

        var tmp = *inPtr;
        tmp.z *= -1;  // ← ONLY Z-AXIS FLIP
        *outPtr = tmp;
    }
}
```

### Tangent Conversion - Float16 (ExportJobs.cs:188-200)
```csharp
[BurstCompile]
public unsafe struct ConvertTangentHalfJob : IJobParallelFor
{
    public uint inputByteStride;
    public uint outputByteStride;

    [ReadOnly]
    [NativeDisableUnsafePtrRestriction]
    public byte* input;

    [WriteOnly]
    [NativeDisableUnsafePtrRestriction]
    public byte* output;

    public void Execute(int i)
    {
        var inPtr = (half4*)(input + i * inputByteStride);
        var outPtr = (float4*)(output + i * outputByteStride);

        var tmp = (float4)(*inPtr);
        tmp.z *= -1;  // ← ONLY Z-AXIS FLIP
        *outPtr = tmp;
    }
}
```

---

## Index Conversion

### Triangle Index Flip - UInt16 (ExportJobs.cs:20-38)
```csharp
[BurstCompile]
public struct ConvertIndicesFlippedJobUInt16 : IJobParallelFor
{
    [ReadOnly]
    public NativeArray<ushort> input;

    [WriteOnly]
    [NativeDisableParallelForRestriction]
    public NativeArray<ushort> result;

    public int indexStart;
    public ushort baseVertexOffset;

    public void Execute(int i)
    {
        result[i * 3 + 0] = (ushort)(input[i * 3 + 0] + baseVertexOffset);
        result[i * 3 + 1] = (ushort)(input[i * 3 + 2] + baseVertexOffset);  // ← FLIP
        result[i * 3 + 2] = (ushort)(input[i * 3 + 1] + baseVertexOffset);  // ← FLIP
    }
}
```

### Triangle Index Flip - UInt32 (ExportJobs.cs:42-59)
```csharp
[BurstCompile]
public struct ConvertIndicesFlippedJobUInt32 : IJobParallelFor
{
    [ReadOnly]
    public NativeArray<uint> input;

    [WriteOnly]
    [NativeDisableParallelForRestriction]
    public NativeArray<uint> result;

    public uint baseVertexOffset;

    public void Execute(int index)
    {
        result[index * 3 + 0] = input[index * 3 + 0] + baseVertexOffset;
        result[index * 3 + 1] = input[index * 3 + 2] + baseVertexOffset;  // ← FLIP
        result[index * 3 + 2] = input[index * 3 + 1] + baseVertexOffset;  // ← FLIP
    }
}
```

---

## Mesh Bounds Calculation

### Bounds with X-Axis Flip (GltfWriter.cs:1106-1110)
```csharp
case VertexAttribute.Position:
    var bounds = uMesh.bounds;
    var max = bounds.max;
    var min = bounds.min;
    accessor.min = new[] { -max.x, min.y, min.z };  // ← X-BOUNDS NEGATED
    accessor.max = new[] { -min.x, max.y, max.z };  // ← X-BOUNDS NEGATED
    attributes.POSITION = accessorId;
    break;
```

---

## Bind Pose Matrix Conversion

### Matrix Column Flip for Skinning (ExportJobs.cs:289-304)
```csharp
public struct ConvertMatrixJob : IJobParallelFor
{
    public NativeArray<float4x4> matrices;

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
}
```

---

## Mesh Export Pipeline

### AddMeshToNode (GltfWriter.cs:173-232)
```csharp
public void AddMeshToNode(
    int nodeId,
    UnityEngine.Mesh uMesh,
    int[] materialIds,
    uint[] joints
    )
{
    if ((m_Settings.ComponentMask & ComponentType.Mesh) == 0) return;
    CertifyNotDisposed();
    var node = m_Nodes[nodeId];

    // Always export positions.
    var attributeUsage = VertexAttributeUsage.Position;
    var skinning = joints != null && joints.Length > 0;
    if (skinning)
    {
        attributeUsage |= VertexAttributeUsage.Skinning;
    }
    // ... material handling ...

    node.mesh = AddMesh(uMesh, attributeUsage);
    if (skinning)
    {
        node.skin = AddSkin(node.mesh, joints);
    }
}
```

### BakeMesh - Vertex Attribute Conversion (GltfWriter.cs:1250-1329)
```csharp
foreach (var pair in attrDataDict)
{
    var vertexAttribute = pair.Key;
    var attrData = pair.Value;
    switch (vertexAttribute)
    {
        case VertexAttribute.Position:
        case VertexAttribute.Normal:
            await ConvertPositionAttribute(
                attrData,
                (uint)inputStrides[attrData.descriptor.stream],
                (uint)outputStrides[attrData.descriptor.stream],
                vertexCount,
                inputStreams[attrData.descriptor.stream],
                outputStreams[attrData.descriptor.stream]
                );
            break;
        case VertexAttribute.Tangent:
            await ConvertTangentAttribute(
                attrData,
                (uint)inputStrides[attrData.descriptor.stream],
                (uint)outputStrides[attrData.descriptor.stream],
                vertexCount,
                inputStreams[attrData.descriptor.stream],
                outputStreams[attrData.descriptor.stream]
                );
            break;
        // ... other attributes ...
    }
}
```

---

## Summary Table

| Operation | File | Lines | Transformation |
|-----------|------|-------|-----------------|
| Root node transform | GameObjectExport.cs | 207-211 | localToWorldMatrix × sceneOrigin |
| Child node transform | GameObjectExport.cs | 215-218 | localPosition/Rotation/Scale |
| Position (float32) | ExportJobs.cs | 110-133 | X *= -1 |
| Position (float16) | ExportJobs.cs | 136-159 | X *= -1 |
| Normal (float32) | ExportJobs.cs | 110-133 | X *= -1 |
| Normal (float16) | ExportJobs.cs | 136-159 | X *= -1 |
| Tangent (float32) | ExportJobs.cs | 162-185 | Z *= -1 |
| Tangent (float16) | ExportJobs.cs | 188-200 | Z *= -1 |
| TexCoord | ExportJobs.cs | (generic) | None |
| Triangle indices | ExportJobs.cs | 20-59 | [0,1,2] → [0,2,1] |
| Mesh bounds | GltfWriter.cs | 1106-1110 | X-bounds negated |
| Bind poses | ExportJobs.cs | 289-304 | Matrix columns flipped |

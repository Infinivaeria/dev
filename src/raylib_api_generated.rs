#![allow(dead_code, non_snake_case, unused_variables, unused_unsafe, clippy::useless_conversion)]
use super::*;
fn read_Vector2(value: Value) -> Result<ffi::Vector2, Error> {
let fields = sequence(value, 2)?;
Ok(ffi::Vector2 { x: f64::try_convert(fields[0])? as f32, y: f64::try_convert(fields[1])? as f32 }) }
fn write_Vector2(ruby: &Ruby, value: ffi::Vector2) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.x).into_value_with(ruby))?;
array.push((value.y).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_Vector3(value: Value) -> Result<ffi::Vector3, Error> {
let fields = sequence(value, 3)?;
Ok(ffi::Vector3 { x: f64::try_convert(fields[0])? as f32, y: f64::try_convert(fields[1])? as f32, z: f64::try_convert(fields[2])? as f32 }) }
fn write_Vector3(ruby: &Ruby, value: ffi::Vector3) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.x).into_value_with(ruby))?;
array.push((value.y).into_value_with(ruby))?;
array.push((value.z).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_Vector4(value: Value) -> Result<ffi::Vector4, Error> {
let fields = sequence(value, 4)?;
Ok(ffi::Vector4 { x: f64::try_convert(fields[0])? as f32, y: f64::try_convert(fields[1])? as f32, z: f64::try_convert(fields[2])? as f32, w: f64::try_convert(fields[3])? as f32 }) }
fn write_Vector4(ruby: &Ruby, value: ffi::Vector4) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.x).into_value_with(ruby))?;
array.push((value.y).into_value_with(ruby))?;
array.push((value.z).into_value_with(ruby))?;
array.push((value.w).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_Matrix(value: Value) -> Result<ffi::Matrix, Error> {
let fields = sequence(value, 16)?;
Ok(ffi::Matrix { m0: f64::try_convert(fields[0])? as f32, m4: f64::try_convert(fields[1])? as f32, m8: f64::try_convert(fields[2])? as f32, m12: f64::try_convert(fields[3])? as f32, m1: f64::try_convert(fields[4])? as f32, m5: f64::try_convert(fields[5])? as f32, m9: f64::try_convert(fields[6])? as f32, m13: f64::try_convert(fields[7])? as f32, m2: f64::try_convert(fields[8])? as f32, m6: f64::try_convert(fields[9])? as f32, m10: f64::try_convert(fields[10])? as f32, m14: f64::try_convert(fields[11])? as f32, m3: f64::try_convert(fields[12])? as f32, m7: f64::try_convert(fields[13])? as f32, m11: f64::try_convert(fields[14])? as f32, m15: f64::try_convert(fields[15])? as f32 }) }
fn write_Matrix(ruby: &Ruby, value: ffi::Matrix) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.m0).into_value_with(ruby))?;
array.push((value.m4).into_value_with(ruby))?;
array.push((value.m8).into_value_with(ruby))?;
array.push((value.m12).into_value_with(ruby))?;
array.push((value.m1).into_value_with(ruby))?;
array.push((value.m5).into_value_with(ruby))?;
array.push((value.m9).into_value_with(ruby))?;
array.push((value.m13).into_value_with(ruby))?;
array.push((value.m2).into_value_with(ruby))?;
array.push((value.m6).into_value_with(ruby))?;
array.push((value.m10).into_value_with(ruby))?;
array.push((value.m14).into_value_with(ruby))?;
array.push((value.m3).into_value_with(ruby))?;
array.push((value.m7).into_value_with(ruby))?;
array.push((value.m11).into_value_with(ruby))?;
array.push((value.m15).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_Color(value: Value) -> Result<ffi::Color, Error> {
let fields = sequence(value, 4)?;
Ok(ffi::Color { r: u8::try_from(u64::try_convert(fields[0])?).map_err(|_| argument("unsigned char out of range"))?, g: u8::try_from(u64::try_convert(fields[1])?).map_err(|_| argument("unsigned char out of range"))?, b: u8::try_from(u64::try_convert(fields[2])?).map_err(|_| argument("unsigned char out of range"))?, a: u8::try_from(u64::try_convert(fields[3])?).map_err(|_| argument("unsigned char out of range"))? }) }
fn write_Color(ruby: &Ruby, value: ffi::Color) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.r).into_value_with(ruby))?;
array.push((value.g).into_value_with(ruby))?;
array.push((value.b).into_value_with(ruby))?;
array.push((value.a).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_Rectangle(value: Value) -> Result<ffi::Rectangle, Error> {
let fields = sequence(value, 4)?;
Ok(ffi::Rectangle { x: f64::try_convert(fields[0])? as f32, y: f64::try_convert(fields[1])? as f32, width: f64::try_convert(fields[2])? as f32, height: f64::try_convert(fields[3])? as f32 }) }
fn write_Rectangle(ruby: &Ruby, value: ffi::Rectangle) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.x).into_value_with(ruby))?;
array.push((value.y).into_value_with(ruby))?;
array.push((value.width).into_value_with(ruby))?;
array.push((value.height).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_Image(value: Value) -> Result<ffi::Image, Error> {
let fields = sequence(value, 5)?;
Ok(ffi::Image { data: address(fields[0])? as _, width: i32::try_from(i64::try_convert(fields[1])?).map_err(|_| argument("int out of range"))?, height: i32::try_from(i64::try_convert(fields[2])?).map_err(|_| argument("int out of range"))?, mipmaps: i32::try_from(i64::try_convert(fields[3])?).map_err(|_| argument("int out of range"))?, format: i32::try_from(i64::try_convert(fields[4])?).map_err(|_| argument("int out of range"))? }) }
fn write_Image(ruby: &Ruby, value: ffi::Image) -> Result<Value, Error> { let array = ruby.ary_new();
array.push(((value.data) as usize).into_value_with(ruby))?;
array.push((value.width).into_value_with(ruby))?;
array.push((value.height).into_value_with(ruby))?;
array.push((value.mipmaps).into_value_with(ruby))?;
array.push((value.format).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_Texture(value: Value) -> Result<ffi::Texture, Error> {
let fields = sequence(value, 5)?;
Ok(ffi::Texture { id: u32::try_from(u64::try_convert(fields[0])?).map_err(|_| argument("unsigned int out of range"))?, width: i32::try_from(i64::try_convert(fields[1])?).map_err(|_| argument("int out of range"))?, height: i32::try_from(i64::try_convert(fields[2])?).map_err(|_| argument("int out of range"))?, mipmaps: i32::try_from(i64::try_convert(fields[3])?).map_err(|_| argument("int out of range"))?, format: i32::try_from(i64::try_convert(fields[4])?).map_err(|_| argument("int out of range"))? }) }
fn write_Texture(ruby: &Ruby, value: ffi::Texture) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.id).into_value_with(ruby))?;
array.push((value.width).into_value_with(ruby))?;
array.push((value.height).into_value_with(ruby))?;
array.push((value.mipmaps).into_value_with(ruby))?;
array.push((value.format).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_RenderTexture(value: Value) -> Result<ffi::RenderTexture, Error> {
let fields = sequence(value, 3)?;
Ok(ffi::RenderTexture { id: u32::try_from(u64::try_convert(fields[0])?).map_err(|_| argument("unsigned int out of range"))?, texture: read_Texture(fields[1])?, depth: read_Texture(fields[2])? }) }
fn write_RenderTexture(ruby: &Ruby, value: ffi::RenderTexture) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.id).into_value_with(ruby))?;
array.push(write_Texture(ruby, value.texture)?)?;
array.push(write_Texture(ruby, value.depth)?)?;
Ok(array.as_value()) }
fn read_NPatchInfo(value: Value) -> Result<ffi::NPatchInfo, Error> {
let fields = sequence(value, 6)?;
Ok(ffi::NPatchInfo { source: read_Rectangle(fields[0])?, left: i32::try_from(i64::try_convert(fields[1])?).map_err(|_| argument("int out of range"))?, top: i32::try_from(i64::try_convert(fields[2])?).map_err(|_| argument("int out of range"))?, right: i32::try_from(i64::try_convert(fields[3])?).map_err(|_| argument("int out of range"))?, bottom: i32::try_from(i64::try_convert(fields[4])?).map_err(|_| argument("int out of range"))?, layout: i32::try_from(i64::try_convert(fields[5])?).map_err(|_| argument("int out of range"))? }) }
fn write_NPatchInfo(ruby: &Ruby, value: ffi::NPatchInfo) -> Result<Value, Error> { let array = ruby.ary_new();
array.push(write_Rectangle(ruby, value.source)?)?;
array.push((value.left).into_value_with(ruby))?;
array.push((value.top).into_value_with(ruby))?;
array.push((value.right).into_value_with(ruby))?;
array.push((value.bottom).into_value_with(ruby))?;
array.push((value.layout).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_GlyphInfo(value: Value) -> Result<ffi::GlyphInfo, Error> {
let fields = sequence(value, 5)?;
Ok(ffi::GlyphInfo { value: i32::try_from(i64::try_convert(fields[0])?).map_err(|_| argument("int out of range"))?, offsetX: i32::try_from(i64::try_convert(fields[1])?).map_err(|_| argument("int out of range"))?, offsetY: i32::try_from(i64::try_convert(fields[2])?).map_err(|_| argument("int out of range"))?, advanceX: i32::try_from(i64::try_convert(fields[3])?).map_err(|_| argument("int out of range"))?, image: read_Image(fields[4])? }) }
fn write_GlyphInfo(ruby: &Ruby, value: ffi::GlyphInfo) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.value).into_value_with(ruby))?;
array.push((value.offsetX).into_value_with(ruby))?;
array.push((value.offsetY).into_value_with(ruby))?;
array.push((value.advanceX).into_value_with(ruby))?;
array.push(write_Image(ruby, value.image)?)?;
Ok(array.as_value()) }
fn read_Font(value: Value) -> Result<ffi::Font, Error> {
let fields = sequence(value, 6)?;
Ok(ffi::Font { baseSize: i32::try_from(i64::try_convert(fields[0])?).map_err(|_| argument("int out of range"))?, glyphCount: i32::try_from(i64::try_convert(fields[1])?).map_err(|_| argument("int out of range"))?, glyphPadding: i32::try_from(i64::try_convert(fields[2])?).map_err(|_| argument("int out of range"))?, texture: read_Texture(fields[3])?, recs: address(fields[4])? as _, glyphs: address(fields[5])? as _ }) }
fn write_Font(ruby: &Ruby, value: ffi::Font) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.baseSize).into_value_with(ruby))?;
array.push((value.glyphCount).into_value_with(ruby))?;
array.push((value.glyphPadding).into_value_with(ruby))?;
array.push(write_Texture(ruby, value.texture)?)?;
array.push(((value.recs) as usize).into_value_with(ruby))?;
array.push(((value.glyphs) as usize).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_Camera3D(value: Value) -> Result<ffi::Camera3D, Error> {
let fields = sequence(value, 5)?;
Ok(ffi::Camera3D { position: read_Vector3(fields[0])?, target: read_Vector3(fields[1])?, up: read_Vector3(fields[2])?, fovy: f64::try_convert(fields[3])? as f32, projection: i32::try_from(i64::try_convert(fields[4])?).map_err(|_| argument("int out of range"))? }) }
fn write_Camera3D(ruby: &Ruby, value: ffi::Camera3D) -> Result<Value, Error> { let array = ruby.ary_new();
array.push(write_Vector3(ruby, value.position)?)?;
array.push(write_Vector3(ruby, value.target)?)?;
array.push(write_Vector3(ruby, value.up)?)?;
array.push((value.fovy).into_value_with(ruby))?;
array.push((value.projection).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_Camera2D(value: Value) -> Result<ffi::Camera2D, Error> {
let fields = sequence(value, 4)?;
Ok(ffi::Camera2D { offset: read_Vector2(fields[0])?, target: read_Vector2(fields[1])?, rotation: f64::try_convert(fields[2])? as f32, zoom: f64::try_convert(fields[3])? as f32 }) }
fn write_Camera2D(ruby: &Ruby, value: ffi::Camera2D) -> Result<Value, Error> { let array = ruby.ary_new();
array.push(write_Vector2(ruby, value.offset)?)?;
array.push(write_Vector2(ruby, value.target)?)?;
array.push((value.rotation).into_value_with(ruby))?;
array.push((value.zoom).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_Mesh(value: Value) -> Result<ffi::Mesh, Error> {
let fields = sequence(value, 16)?;
Ok(ffi::Mesh { vertexCount: i32::try_from(i64::try_convert(fields[0])?).map_err(|_| argument("int out of range"))?, triangleCount: i32::try_from(i64::try_convert(fields[1])?).map_err(|_| argument("int out of range"))?, vertices: address(fields[2])? as _, texcoords: address(fields[3])? as _, texcoords2: address(fields[4])? as _, normals: address(fields[5])? as _, tangents: address(fields[6])? as _, colors: address(fields[7])? as _, indices: address(fields[8])? as _, boneCount: i32::try_from(i64::try_convert(fields[9])?).map_err(|_| argument("int out of range"))?, boneIndices: address(fields[10])? as _, boneWeights: address(fields[11])? as _, animVertices: address(fields[12])? as _, animNormals: address(fields[13])? as _, vaoId: u32::try_from(u64::try_convert(fields[14])?).map_err(|_| argument("unsigned int out of range"))?, vboId: address(fields[15])? as _ }) }
fn write_Mesh(ruby: &Ruby, value: ffi::Mesh) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.vertexCount).into_value_with(ruby))?;
array.push((value.triangleCount).into_value_with(ruby))?;
array.push(((value.vertices) as usize).into_value_with(ruby))?;
array.push(((value.texcoords) as usize).into_value_with(ruby))?;
array.push(((value.texcoords2) as usize).into_value_with(ruby))?;
array.push(((value.normals) as usize).into_value_with(ruby))?;
array.push(((value.tangents) as usize).into_value_with(ruby))?;
array.push(((value.colors) as usize).into_value_with(ruby))?;
array.push(((value.indices) as usize).into_value_with(ruby))?;
array.push((value.boneCount).into_value_with(ruby))?;
array.push(((value.boneIndices) as usize).into_value_with(ruby))?;
array.push(((value.boneWeights) as usize).into_value_with(ruby))?;
array.push(((value.animVertices) as usize).into_value_with(ruby))?;
array.push(((value.animNormals) as usize).into_value_with(ruby))?;
array.push((value.vaoId).into_value_with(ruby))?;
array.push(((value.vboId) as usize).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_Shader(value: Value) -> Result<ffi::Shader, Error> {
let fields = sequence(value, 2)?;
Ok(ffi::Shader { id: u32::try_from(u64::try_convert(fields[0])?).map_err(|_| argument("unsigned int out of range"))?, locs: address(fields[1])? as _ }) }
fn write_Shader(ruby: &Ruby, value: ffi::Shader) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.id).into_value_with(ruby))?;
array.push(((value.locs) as usize).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_MaterialMap(value: Value) -> Result<ffi::MaterialMap, Error> {
let fields = sequence(value, 3)?;
Ok(ffi::MaterialMap { texture: read_Texture(fields[0])?, color: read_Color(fields[1])?, value: f64::try_convert(fields[2])? as f32 }) }
fn write_MaterialMap(ruby: &Ruby, value: ffi::MaterialMap) -> Result<Value, Error> { let array = ruby.ary_new();
array.push(write_Texture(ruby, value.texture)?)?;
array.push(write_Color(ruby, value.color)?)?;
array.push((value.value).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_Material(value: Value) -> Result<ffi::Material, Error> {
let fields = sequence(value, 3)?;
Ok(ffi::Material { shader: read_Shader(fields[0])?, maps: address(fields[1])? as _, params: { let values = sequence(fields[2], 4)?; [f64::try_convert(values[0])? as f32, f64::try_convert(values[1])? as f32, f64::try_convert(values[2])? as f32, f64::try_convert(values[3])? as f32] } }) }
fn write_Material(ruby: &Ruby, value: ffi::Material) -> Result<Value, Error> { let array = ruby.ary_new();
array.push(write_Shader(ruby, value.shader)?)?;
array.push(((value.maps) as usize).into_value_with(ruby))?;
array.push({ let array = ruby.ary_new(); for element in value.params { array.push((element).into_value_with(ruby))?; } array.as_value() })?;
Ok(array.as_value()) }
fn read_Transform(value: Value) -> Result<ffi::Transform, Error> {
let fields = sequence(value, 3)?;
Ok(ffi::Transform { translation: read_Vector3(fields[0])?, rotation: read_Vector4(fields[1])?.into(), scale: read_Vector3(fields[2])? }) }
fn write_Transform(ruby: &Ruby, value: ffi::Transform) -> Result<Value, Error> { let array = ruby.ary_new();
array.push(write_Vector3(ruby, value.translation)?)?;
array.push(write_Vector4(ruby, value.rotation.into())?)?;
array.push(write_Vector3(ruby, value.scale)?)?;
Ok(array.as_value()) }
fn read_BoneInfo(value: Value) -> Result<ffi::BoneInfo, Error> {
let fields = sequence(value, 2)?;
Ok(ffi::BoneInfo { name: { let values = sequence(fields[0], 32)?; [std::ffi::c_char::try_from(i64::try_convert(values[0])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[1])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[2])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[3])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[4])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[5])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[6])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[7])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[8])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[9])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[10])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[11])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[12])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[13])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[14])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[15])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[16])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[17])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[18])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[19])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[20])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[21])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[22])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[23])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[24])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[25])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[26])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[27])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[28])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[29])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[30])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[31])?).map_err(|_| argument("char out of range"))?] }, parent: i32::try_from(i64::try_convert(fields[1])?).map_err(|_| argument("int out of range"))? }) }
fn write_BoneInfo(ruby: &Ruby, value: ffi::BoneInfo) -> Result<Value, Error> { let array = ruby.ary_new();
array.push({ let array = ruby.ary_new(); for element in value.name { array.push((element).into_value_with(ruby))?; } array.as_value() })?;
array.push((value.parent).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_ModelSkeleton(value: Value) -> Result<ffi::ModelSkeleton, Error> {
let fields = sequence(value, 3)?;
Ok(ffi::ModelSkeleton { boneCount: i32::try_from(i64::try_convert(fields[0])?).map_err(|_| argument("int out of range"))?, bones: address(fields[1])? as _, bindPose: address(fields[2])? as _ }) }
fn write_ModelSkeleton(ruby: &Ruby, value: ffi::ModelSkeleton) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.boneCount).into_value_with(ruby))?;
array.push(((value.bones) as usize).into_value_with(ruby))?;
array.push(((value.bindPose) as usize).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_Model(value: Value) -> Result<ffi::Model, Error> {
let fields = sequence(value, 9)?;
Ok(ffi::Model { transform: read_Matrix(fields[0])?, meshCount: i32::try_from(i64::try_convert(fields[1])?).map_err(|_| argument("int out of range"))?, materialCount: i32::try_from(i64::try_convert(fields[2])?).map_err(|_| argument("int out of range"))?, meshes: address(fields[3])? as _, materials: address(fields[4])? as _, meshMaterial: address(fields[5])? as _, skeleton: read_ModelSkeleton(fields[6])?, currentPose: address(fields[7])? as _, boneMatrices: address(fields[8])? as _ }) }
fn write_Model(ruby: &Ruby, value: ffi::Model) -> Result<Value, Error> { let array = ruby.ary_new();
array.push(write_Matrix(ruby, value.transform)?)?;
array.push((value.meshCount).into_value_with(ruby))?;
array.push((value.materialCount).into_value_with(ruby))?;
array.push(((value.meshes) as usize).into_value_with(ruby))?;
array.push(((value.materials) as usize).into_value_with(ruby))?;
array.push(((value.meshMaterial) as usize).into_value_with(ruby))?;
array.push(write_ModelSkeleton(ruby, value.skeleton)?)?;
array.push(((value.currentPose) as usize).into_value_with(ruby))?;
array.push(((value.boneMatrices) as usize).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_ModelAnimation(value: Value) -> Result<ffi::ModelAnimation, Error> {
let fields = sequence(value, 4)?;
Ok(ffi::ModelAnimation { name: { let values = sequence(fields[0], 32)?; [std::ffi::c_char::try_from(i64::try_convert(values[0])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[1])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[2])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[3])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[4])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[5])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[6])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[7])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[8])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[9])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[10])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[11])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[12])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[13])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[14])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[15])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[16])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[17])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[18])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[19])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[20])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[21])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[22])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[23])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[24])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[25])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[26])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[27])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[28])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[29])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[30])?).map_err(|_| argument("char out of range"))?, std::ffi::c_char::try_from(i64::try_convert(values[31])?).map_err(|_| argument("char out of range"))?] }, boneCount: i32::try_from(i64::try_convert(fields[1])?).map_err(|_| argument("int out of range"))?, keyframeCount: i32::try_from(i64::try_convert(fields[2])?).map_err(|_| argument("int out of range"))?, keyframePoses: address(fields[3])? as _ }) }
fn write_ModelAnimation(ruby: &Ruby, value: ffi::ModelAnimation) -> Result<Value, Error> { let array = ruby.ary_new();
array.push({ let array = ruby.ary_new(); for element in value.name { array.push((element).into_value_with(ruby))?; } array.as_value() })?;
array.push((value.boneCount).into_value_with(ruby))?;
array.push((value.keyframeCount).into_value_with(ruby))?;
array.push(((value.keyframePoses) as usize).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_Ray(value: Value) -> Result<ffi::Ray, Error> {
let fields = sequence(value, 2)?;
Ok(ffi::Ray { position: read_Vector3(fields[0])?, direction: read_Vector3(fields[1])? }) }
fn write_Ray(ruby: &Ruby, value: ffi::Ray) -> Result<Value, Error> { let array = ruby.ary_new();
array.push(write_Vector3(ruby, value.position)?)?;
array.push(write_Vector3(ruby, value.direction)?)?;
Ok(array.as_value()) }
fn read_RayCollision(value: Value) -> Result<ffi::RayCollision, Error> {
let fields = sequence(value, 4)?;
Ok(ffi::RayCollision { hit: bool::try_convert(fields[0])?, distance: f64::try_convert(fields[1])? as f32, point: read_Vector3(fields[2])?, normal: read_Vector3(fields[3])? }) }
fn write_RayCollision(ruby: &Ruby, value: ffi::RayCollision) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.hit).into_value_with(ruby))?;
array.push((value.distance).into_value_with(ruby))?;
array.push(write_Vector3(ruby, value.point)?)?;
array.push(write_Vector3(ruby, value.normal)?)?;
Ok(array.as_value()) }
fn read_BoundingBox(value: Value) -> Result<ffi::BoundingBox, Error> {
let fields = sequence(value, 2)?;
Ok(ffi::BoundingBox { min: read_Vector3(fields[0])?, max: read_Vector3(fields[1])? }) }
fn write_BoundingBox(ruby: &Ruby, value: ffi::BoundingBox) -> Result<Value, Error> { let array = ruby.ary_new();
array.push(write_Vector3(ruby, value.min)?)?;
array.push(write_Vector3(ruby, value.max)?)?;
Ok(array.as_value()) }
fn read_Wave(value: Value) -> Result<ffi::Wave, Error> {
let fields = sequence(value, 5)?;
Ok(ffi::Wave { frameCount: u32::try_from(u64::try_convert(fields[0])?).map_err(|_| argument("unsigned int out of range"))?, sampleRate: u32::try_from(u64::try_convert(fields[1])?).map_err(|_| argument("unsigned int out of range"))?, sampleSize: u32::try_from(u64::try_convert(fields[2])?).map_err(|_| argument("unsigned int out of range"))?, channels: u32::try_from(u64::try_convert(fields[3])?).map_err(|_| argument("unsigned int out of range"))?, data: address(fields[4])? as _ }) }
fn write_Wave(ruby: &Ruby, value: ffi::Wave) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.frameCount).into_value_with(ruby))?;
array.push((value.sampleRate).into_value_with(ruby))?;
array.push((value.sampleSize).into_value_with(ruby))?;
array.push((value.channels).into_value_with(ruby))?;
array.push(((value.data) as usize).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_AudioStream(value: Value) -> Result<ffi::AudioStream, Error> {
let fields = sequence(value, 5)?;
Ok(ffi::AudioStream { buffer: address(fields[0])? as _, processor: address(fields[1])? as _, sampleRate: u32::try_from(u64::try_convert(fields[2])?).map_err(|_| argument("unsigned int out of range"))?, sampleSize: u32::try_from(u64::try_convert(fields[3])?).map_err(|_| argument("unsigned int out of range"))?, channels: u32::try_from(u64::try_convert(fields[4])?).map_err(|_| argument("unsigned int out of range"))? }) }
fn write_AudioStream(ruby: &Ruby, value: ffi::AudioStream) -> Result<Value, Error> { let array = ruby.ary_new();
array.push(((value.buffer) as usize).into_value_with(ruby))?;
array.push(((value.processor) as usize).into_value_with(ruby))?;
array.push((value.sampleRate).into_value_with(ruby))?;
array.push((value.sampleSize).into_value_with(ruby))?;
array.push((value.channels).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_Sound(value: Value) -> Result<ffi::Sound, Error> {
let fields = sequence(value, 2)?;
Ok(ffi::Sound { stream: read_AudioStream(fields[0])?, frameCount: u32::try_from(u64::try_convert(fields[1])?).map_err(|_| argument("unsigned int out of range"))? }) }
fn write_Sound(ruby: &Ruby, value: ffi::Sound) -> Result<Value, Error> { let array = ruby.ary_new();
array.push(write_AudioStream(ruby, value.stream)?)?;
array.push((value.frameCount).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_Music(value: Value) -> Result<ffi::Music, Error> {
let fields = sequence(value, 5)?;
Ok(ffi::Music { stream: read_AudioStream(fields[0])?, frameCount: u32::try_from(u64::try_convert(fields[1])?).map_err(|_| argument("unsigned int out of range"))?, looping: bool::try_convert(fields[2])?, ctxType: i32::try_from(i64::try_convert(fields[3])?).map_err(|_| argument("int out of range"))?, ctxData: address(fields[4])? as _ }) }
fn write_Music(ruby: &Ruby, value: ffi::Music) -> Result<Value, Error> { let array = ruby.ary_new();
array.push(write_AudioStream(ruby, value.stream)?)?;
array.push((value.frameCount).into_value_with(ruby))?;
array.push((value.looping).into_value_with(ruby))?;
array.push((value.ctxType).into_value_with(ruby))?;
array.push(((value.ctxData) as usize).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_VrDeviceInfo(value: Value) -> Result<ffi::VrDeviceInfo, Error> {
let fields = sequence(value, 9)?;
Ok(ffi::VrDeviceInfo { hResolution: i32::try_from(i64::try_convert(fields[0])?).map_err(|_| argument("int out of range"))?, vResolution: i32::try_from(i64::try_convert(fields[1])?).map_err(|_| argument("int out of range"))?, hScreenSize: f64::try_convert(fields[2])? as f32, vScreenSize: f64::try_convert(fields[3])? as f32, eyeToScreenDistance: f64::try_convert(fields[4])? as f32, lensSeparationDistance: f64::try_convert(fields[5])? as f32, interpupillaryDistance: f64::try_convert(fields[6])? as f32, lensDistortionValues: { let values = sequence(fields[7], 4)?; [f64::try_convert(values[0])? as f32, f64::try_convert(values[1])? as f32, f64::try_convert(values[2])? as f32, f64::try_convert(values[3])? as f32] }, chromaAbCorrection: { let values = sequence(fields[8], 4)?; [f64::try_convert(values[0])? as f32, f64::try_convert(values[1])? as f32, f64::try_convert(values[2])? as f32, f64::try_convert(values[3])? as f32] } }) }
fn write_VrDeviceInfo(ruby: &Ruby, value: ffi::VrDeviceInfo) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.hResolution).into_value_with(ruby))?;
array.push((value.vResolution).into_value_with(ruby))?;
array.push((value.hScreenSize).into_value_with(ruby))?;
array.push((value.vScreenSize).into_value_with(ruby))?;
array.push((value.eyeToScreenDistance).into_value_with(ruby))?;
array.push((value.lensSeparationDistance).into_value_with(ruby))?;
array.push((value.interpupillaryDistance).into_value_with(ruby))?;
array.push({ let array = ruby.ary_new(); for element in value.lensDistortionValues { array.push((element).into_value_with(ruby))?; } array.as_value() })?;
array.push({ let array = ruby.ary_new(); for element in value.chromaAbCorrection { array.push((element).into_value_with(ruby))?; } array.as_value() })?;
Ok(array.as_value()) }
fn read_VrStereoConfig(value: Value) -> Result<ffi::VrStereoConfig, Error> {
let fields = sequence(value, 8)?;
Ok(ffi::VrStereoConfig { projection: { let values = sequence(fields[0], 2)?; [read_Matrix(values[0])?, read_Matrix(values[1])?] }, viewOffset: { let values = sequence(fields[1], 2)?; [read_Matrix(values[0])?, read_Matrix(values[1])?] }, leftLensCenter: { let values = sequence(fields[2], 2)?; [f64::try_convert(values[0])? as f32, f64::try_convert(values[1])? as f32] }, rightLensCenter: { let values = sequence(fields[3], 2)?; [f64::try_convert(values[0])? as f32, f64::try_convert(values[1])? as f32] }, leftScreenCenter: { let values = sequence(fields[4], 2)?; [f64::try_convert(values[0])? as f32, f64::try_convert(values[1])? as f32] }, rightScreenCenter: { let values = sequence(fields[5], 2)?; [f64::try_convert(values[0])? as f32, f64::try_convert(values[1])? as f32] }, scale: { let values = sequence(fields[6], 2)?; [f64::try_convert(values[0])? as f32, f64::try_convert(values[1])? as f32] }, scaleIn: { let values = sequence(fields[7], 2)?; [f64::try_convert(values[0])? as f32, f64::try_convert(values[1])? as f32] } }) }
fn write_VrStereoConfig(ruby: &Ruby, value: ffi::VrStereoConfig) -> Result<Value, Error> { let array = ruby.ary_new();
array.push({ let array = ruby.ary_new(); for element in value.projection { array.push(write_Matrix(ruby, element)?)?; } array.as_value() })?;
array.push({ let array = ruby.ary_new(); for element in value.viewOffset { array.push(write_Matrix(ruby, element)?)?; } array.as_value() })?;
array.push({ let array = ruby.ary_new(); for element in value.leftLensCenter { array.push((element).into_value_with(ruby))?; } array.as_value() })?;
array.push({ let array = ruby.ary_new(); for element in value.rightLensCenter { array.push((element).into_value_with(ruby))?; } array.as_value() })?;
array.push({ let array = ruby.ary_new(); for element in value.leftScreenCenter { array.push((element).into_value_with(ruby))?; } array.as_value() })?;
array.push({ let array = ruby.ary_new(); for element in value.rightScreenCenter { array.push((element).into_value_with(ruby))?; } array.as_value() })?;
array.push({ let array = ruby.ary_new(); for element in value.scale { array.push((element).into_value_with(ruby))?; } array.as_value() })?;
array.push({ let array = ruby.ary_new(); for element in value.scaleIn { array.push((element).into_value_with(ruby))?; } array.as_value() })?;
Ok(array.as_value()) }
fn read_FilePathList(value: Value) -> Result<ffi::FilePathList, Error> {
let fields = sequence(value, 2)?;
Ok(ffi::FilePathList { count: u32::try_from(u64::try_convert(fields[0])?).map_err(|_| argument("unsigned int out of range"))?, paths: address(fields[1])? as _ }) }
fn write_FilePathList(ruby: &Ruby, value: ffi::FilePathList) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.count).into_value_with(ruby))?;
array.push(((value.paths) as usize).into_value_with(ruby))?;
Ok(array.as_value()) }
fn read_AutomationEvent(value: Value) -> Result<ffi::AutomationEvent, Error> {
let fields = sequence(value, 3)?;
Ok(ffi::AutomationEvent { frame: u32::try_from(u64::try_convert(fields[0])?).map_err(|_| argument("unsigned int out of range"))?, type_: u32::try_from(u64::try_convert(fields[1])?).map_err(|_| argument("unsigned int out of range"))?, params: { let values = sequence(fields[2], 4)?; [i32::try_from(i64::try_convert(values[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(values[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(values[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(values[3])?).map_err(|_| argument("int out of range"))?] } }) }
fn write_AutomationEvent(ruby: &Ruby, value: ffi::AutomationEvent) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.frame).into_value_with(ruby))?;
array.push((value.type_).into_value_with(ruby))?;
array.push({ let array = ruby.ary_new(); for element in value.params { array.push((element).into_value_with(ruby))?; } array.as_value() })?;
Ok(array.as_value()) }
fn read_AutomationEventList(value: Value) -> Result<ffi::AutomationEventList, Error> {
let fields = sequence(value, 3)?;
Ok(ffi::AutomationEventList { capacity: u32::try_from(u64::try_convert(fields[0])?).map_err(|_| argument("unsigned int out of range"))?, count: u32::try_from(u64::try_convert(fields[1])?).map_err(|_| argument("unsigned int out of range"))?, events: address(fields[2])? as _ }) }
fn write_AutomationEventList(ruby: &Ruby, value: ffi::AutomationEventList) -> Result<Value, Error> { let array = ruby.ary_new();
array.push((value.capacity).into_value_with(ruby))?;
array.push((value.count).into_value_with(ruby))?;
array.push(((value.events) as usize).into_value_with(ruby))?;
Ok(array.as_value()) }
pub(super) fn call(ruby: &Ruby, name: &str, args: &[Value]) -> Result<Value, Error> { match name {
"InitWindow" => { arity(args, 3)?;
let text_2 = string_argument(args[2])?;
let result = crate::game::init_window(i32::try_convert(args[0])?, i32::try_convert(args[1])?, String::try_convert(args[2])?)?;
Ok(ruby.qnil().as_value())
},
"CloseWindow" => { arity(args, 0)?;
let result = crate::game::close_window()?;
Ok(ruby.qnil().as_value())
},
"WindowShouldClose" => { arity(args, 0)?;
let result = crate::game::window_should_close()?;
Ok((result).into_value_with(ruby))
},
"IsWindowReady" => { arity(args, 0)?;
let result = unsafe { ffi::IsWindowReady() };
Ok((result).into_value_with(ruby))
},
"IsWindowFullscreen" => { arity(args, 0)?;
let result = unsafe { ffi::IsWindowFullscreen() };
Ok((result).into_value_with(ruby))
},
"IsWindowHidden" => { arity(args, 0)?;
let result = unsafe { ffi::IsWindowHidden() };
Ok((result).into_value_with(ruby))
},
"IsWindowMinimized" => { arity(args, 0)?;
let result = unsafe { ffi::IsWindowMinimized() };
Ok((result).into_value_with(ruby))
},
"IsWindowMaximized" => { arity(args, 0)?;
let result = unsafe { ffi::IsWindowMaximized() };
Ok((result).into_value_with(ruby))
},
"IsWindowFocused" => { arity(args, 0)?;
let result = unsafe { ffi::IsWindowFocused() };
Ok((result).into_value_with(ruby))
},
"IsWindowResized" => { arity(args, 0)?;
let result = unsafe { ffi::IsWindowResized() };
Ok((result).into_value_with(ruby))
},
"IsWindowState" => { arity(args, 1)?;
let result = unsafe { ffi::IsWindowState(u32::try_from(u64::try_convert(args[0])?).map_err(|_| argument("unsigned int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"SetWindowState" => { arity(args, 1)?;
let result = unsafe { ffi::SetWindowState(u32::try_from(u64::try_convert(args[0])?).map_err(|_| argument("unsigned int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"ClearWindowState" => { arity(args, 1)?;
let result = unsafe { ffi::ClearWindowState(u32::try_from(u64::try_convert(args[0])?).map_err(|_| argument("unsigned int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"ToggleFullscreen" => { arity(args, 0)?;
let result = unsafe { ffi::ToggleFullscreen() };
Ok(ruby.qnil().as_value())
},
"ToggleBorderlessWindowed" => { arity(args, 0)?;
let result = unsafe { ffi::ToggleBorderlessWindowed() };
Ok(ruby.qnil().as_value())
},
"MaximizeWindow" => { arity(args, 0)?;
let result = unsafe { ffi::MaximizeWindow() };
Ok(ruby.qnil().as_value())
},
"MinimizeWindow" => { arity(args, 0)?;
let result = unsafe { ffi::MinimizeWindow() };
Ok(ruby.qnil().as_value())
},
"RestoreWindow" => { arity(args, 0)?;
let result = unsafe { ffi::RestoreWindow() };
Ok(ruby.qnil().as_value())
},
"SetWindowIcon" => { arity(args, 1)?;
let result = unsafe { ffi::SetWindowIcon(read_Image(args[0])?) };
Ok(ruby.qnil().as_value())
},
"SetWindowIcons" => { arity(args, 2)?;
let result = unsafe { ffi::SetWindowIcons(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"SetWindowTitle" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::SetWindowTitle(text_0.pointer()) };
Ok(ruby.qnil().as_value())
},
"SetWindowPosition" => { arity(args, 2)?;
let result = unsafe { ffi::SetWindowPosition(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"SetWindowMonitor" => { arity(args, 1)?;
let result = unsafe { ffi::SetWindowMonitor(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"SetWindowMinSize" => { arity(args, 2)?;
let result = unsafe { ffi::SetWindowMinSize(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"SetWindowMaxSize" => { arity(args, 2)?;
let result = unsafe { ffi::SetWindowMaxSize(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"SetWindowSize" => { arity(args, 2)?;
let result = unsafe { ffi::SetWindowSize(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"SetWindowOpacity" => { arity(args, 1)?;
let result = unsafe { ffi::SetWindowOpacity(f64::try_convert(args[0])? as f32) };
Ok(ruby.qnil().as_value())
},
"SetWindowFocused" => { arity(args, 0)?;
let result = unsafe { ffi::SetWindowFocused() };
Ok(ruby.qnil().as_value())
},
"GetWindowHandle" => { arity(args, 0)?;
let result = unsafe { ffi::GetWindowHandle() };
Ok(((result) as usize).into_value_with(ruby))
},
"GetScreenWidth" => { arity(args, 0)?;
let result = unsafe { ffi::GetScreenWidth() };
Ok((result).into_value_with(ruby))
},
"GetScreenHeight" => { arity(args, 0)?;
let result = unsafe { ffi::GetScreenHeight() };
Ok((result).into_value_with(ruby))
},
"GetRenderWidth" => { arity(args, 0)?;
let result = unsafe { ffi::GetRenderWidth() };
Ok((result).into_value_with(ruby))
},
"GetRenderHeight" => { arity(args, 0)?;
let result = unsafe { ffi::GetRenderHeight() };
Ok((result).into_value_with(ruby))
},
"GetMonitorCount" => { arity(args, 0)?;
let result = unsafe { ffi::GetMonitorCount() };
Ok((result).into_value_with(ruby))
},
"GetCurrentMonitor" => { arity(args, 0)?;
let result = unsafe { ffi::GetCurrentMonitor() };
Ok((result).into_value_with(ruby))
},
"GetMonitorPosition" => { arity(args, 1)?;
let result = unsafe { ffi::GetMonitorPosition(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Vector2(ruby, result)?)
},
"GetMonitorWidth" => { arity(args, 1)?;
let result = unsafe { ffi::GetMonitorWidth(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"GetMonitorHeight" => { arity(args, 1)?;
let result = unsafe { ffi::GetMonitorHeight(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"GetMonitorPhysicalWidth" => { arity(args, 1)?;
let result = unsafe { ffi::GetMonitorPhysicalWidth(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"GetMonitorPhysicalHeight" => { arity(args, 1)?;
let result = unsafe { ffi::GetMonitorPhysicalHeight(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"GetMonitorRefreshRate" => { arity(args, 1)?;
let result = unsafe { ffi::GetMonitorRefreshRate(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"GetWindowPosition" => { arity(args, 0)?;
let result = unsafe { ffi::GetWindowPosition() };
Ok(write_Vector2(ruby, result)?)
},
"GetWindowScaleDPI" => { arity(args, 0)?;
let result = unsafe { ffi::GetWindowScaleDPI() };
Ok(write_Vector2(ruby, result)?)
},
"GetMonitorName" => { arity(args, 1)?;
let result = unsafe { ffi::GetMonitorName(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok(if result.is_null() { ruby.qnil().as_value() } else { ruby.str_new(&unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy()).as_value() })
},
"SetClipboardText" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::SetClipboardText(text_0.pointer()) };
Ok(ruby.qnil().as_value())
},
"GetClipboardText" => { arity(args, 0)?;
let result = unsafe { ffi::GetClipboardText() };
Ok(if result.is_null() { ruby.qnil().as_value() } else { ruby.str_new(&unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy()).as_value() })
},
"GetClipboardImage" => { arity(args, 0)?;
let result = unsafe { ffi::GetClipboardImage() };
Ok(write_Image(ruby, result)?)
},
"EnableEventWaiting" => { arity(args, 0)?;
let result = unsafe { ffi::EnableEventWaiting() };
Ok(ruby.qnil().as_value())
},
"DisableEventWaiting" => { arity(args, 0)?;
let result = unsafe { ffi::DisableEventWaiting() };
Ok(ruby.qnil().as_value())
},
"ShowCursor" => { arity(args, 0)?;
let result = unsafe { ffi::ShowCursor() };
Ok(ruby.qnil().as_value())
},
"HideCursor" => { arity(args, 0)?;
let result = unsafe { ffi::HideCursor() };
Ok(ruby.qnil().as_value())
},
"IsCursorHidden" => { arity(args, 0)?;
let result = unsafe { ffi::IsCursorHidden() };
Ok((result).into_value_with(ruby))
},
"EnableCursor" => { arity(args, 0)?;
let result = unsafe { ffi::EnableCursor() };
Ok(ruby.qnil().as_value())
},
"DisableCursor" => { arity(args, 0)?;
let result = unsafe { ffi::DisableCursor() };
Ok(ruby.qnil().as_value())
},
"IsCursorOnScreen" => { arity(args, 0)?;
let result = unsafe { ffi::IsCursorOnScreen() };
Ok((result).into_value_with(ruby))
},
"ClearBackground" => { arity(args, 1)?;
let result = unsafe { ffi::ClearBackground(read_Color(args[0])?) };
Ok(ruby.qnil().as_value())
},
"BeginDrawing" => { arity(args, 0)?;
let result = unsafe { ffi::BeginDrawing() };
Ok(ruby.qnil().as_value())
},
"EndDrawing" => { arity(args, 0)?;
let result = crate::game::end_drawing()?;
Ok(ruby.qnil().as_value())
},
"BeginMode2D" => { arity(args, 1)?;
let result = unsafe { ffi::BeginMode2D(read_Camera2D(args[0])?) };
Ok(ruby.qnil().as_value())
},
"EndMode2D" => { arity(args, 0)?;
let result = unsafe { ffi::EndMode2D() };
Ok(ruby.qnil().as_value())
},
"BeginMode3D" => { arity(args, 1)?;
let result = unsafe { ffi::BeginMode3D(read_Camera3D(args[0])?) };
Ok(ruby.qnil().as_value())
},
"EndMode3D" => { arity(args, 0)?;
let result = unsafe { ffi::EndMode3D() };
Ok(ruby.qnil().as_value())
},
"BeginTextureMode" => { arity(args, 1)?;
let result = unsafe { ffi::BeginTextureMode(read_RenderTexture(args[0])?) };
Ok(ruby.qnil().as_value())
},
"EndTextureMode" => { arity(args, 0)?;
let result = unsafe { ffi::EndTextureMode() };
Ok(ruby.qnil().as_value())
},
"BeginShaderMode" => { arity(args, 1)?;
let result = unsafe { ffi::BeginShaderMode(read_Shader(args[0])?) };
Ok(ruby.qnil().as_value())
},
"EndShaderMode" => { arity(args, 0)?;
let result = unsafe { ffi::EndShaderMode() };
Ok(ruby.qnil().as_value())
},
"BeginBlendMode" => { arity(args, 1)?;
let result = unsafe { ffi::BeginBlendMode(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"EndBlendMode" => { arity(args, 0)?;
let result = unsafe { ffi::EndBlendMode() };
Ok(ruby.qnil().as_value())
},
"BeginScissorMode" => { arity(args, 4)?;
let result = unsafe { ffi::BeginScissorMode(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"EndScissorMode" => { arity(args, 0)?;
let result = unsafe { ffi::EndScissorMode() };
Ok(ruby.qnil().as_value())
},
"BeginVrStereoMode" => { arity(args, 1)?;
let result = unsafe { ffi::BeginVrStereoMode(read_VrStereoConfig(args[0])?) };
Ok(ruby.qnil().as_value())
},
"EndVrStereoMode" => { arity(args, 0)?;
let result = unsafe { ffi::EndVrStereoMode() };
Ok(ruby.qnil().as_value())
},
"LoadVrStereoConfig" => { arity(args, 1)?;
let result = unsafe { ffi::LoadVrStereoConfig(read_VrDeviceInfo(args[0])?) };
Ok(write_VrStereoConfig(ruby, result)?)
},
"UnloadVrStereoConfig" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadVrStereoConfig(read_VrStereoConfig(args[0])?) };
Ok(ruby.qnil().as_value())
},
"LoadShader" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::LoadShader(text_0.pointer(), text_1.pointer()) };
Ok(write_Shader(ruby, result)?)
},
"LoadShaderFromMemory" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::LoadShaderFromMemory(text_0.pointer(), text_1.pointer()) };
Ok(write_Shader(ruby, result)?)
},
"IsShaderValid" => { arity(args, 1)?;
let result = unsafe { ffi::IsShaderValid(read_Shader(args[0])?) };
Ok((result).into_value_with(ruby))
},
"GetShaderLocation" => { arity(args, 2)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::GetShaderLocation(read_Shader(args[0])?, text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"GetShaderLocationAttrib" => { arity(args, 2)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::GetShaderLocationAttrib(read_Shader(args[0])?, text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"SetShaderValue" => { arity(args, 4)?;
let result = unsafe { ffi::SetShaderValue(read_Shader(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, address(args[2])? as _, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"SetShaderValueV" => { arity(args, 5)?;
let result = unsafe { ffi::SetShaderValueV(read_Shader(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, address(args[2])? as _, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"SetShaderValueMatrix" => { arity(args, 3)?;
let result = unsafe { ffi::SetShaderValueMatrix(read_Shader(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, read_Matrix(args[2])?) };
Ok(ruby.qnil().as_value())
},
"SetShaderValueTexture" => { arity(args, 3)?;
let result = unsafe { ffi::SetShaderValueTexture(read_Shader(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, read_Texture(args[2])?) };
Ok(ruby.qnil().as_value())
},
"UnloadShader" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadShader(read_Shader(args[0])?) };
Ok(ruby.qnil().as_value())
},
"GetScreenToWorldRay" => { arity(args, 2)?;
let result = unsafe { ffi::GetScreenToWorldRay(read_Vector2(args[0])?, read_Camera3D(args[1])?) };
Ok(write_Ray(ruby, result)?)
},
"GetScreenToWorldRayEx" => { arity(args, 4)?;
let result = unsafe { ffi::GetScreenToWorldRayEx(read_Vector2(args[0])?, read_Camera3D(args[1])?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Ray(ruby, result)?)
},
"GetWorldToScreen" => { arity(args, 2)?;
let result = unsafe { ffi::GetWorldToScreen(read_Vector3(args[0])?, read_Camera3D(args[1])?) };
Ok(write_Vector2(ruby, result)?)
},
"GetWorldToScreenEx" => { arity(args, 4)?;
let result = unsafe { ffi::GetWorldToScreenEx(read_Vector3(args[0])?, read_Camera3D(args[1])?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Vector2(ruby, result)?)
},
"GetWorldToScreen2D" => { arity(args, 2)?;
let result = unsafe { ffi::GetWorldToScreen2D(read_Vector2(args[0])?, read_Camera2D(args[1])?) };
Ok(write_Vector2(ruby, result)?)
},
"GetScreenToWorld2D" => { arity(args, 2)?;
let result = unsafe { ffi::GetScreenToWorld2D(read_Vector2(args[0])?, read_Camera2D(args[1])?) };
Ok(write_Vector2(ruby, result)?)
},
"GetCameraMatrix" => { arity(args, 1)?;
let result = unsafe { ffi::GetCameraMatrix(read_Camera3D(args[0])?) };
Ok(write_Matrix(ruby, result)?)
},
"GetCameraMatrix2D" => { arity(args, 1)?;
let result = unsafe { ffi::GetCameraMatrix2D(read_Camera2D(args[0])?) };
Ok(write_Matrix(ruby, result)?)
},
"SetTargetFPS" => { arity(args, 1)?;
let result = unsafe { ffi::SetTargetFPS(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"GetFrameTime" => { arity(args, 0)?;
let result = unsafe { ffi::GetFrameTime() };
Ok((result).into_value_with(ruby))
},
"GetTime" => { arity(args, 0)?;
let result = unsafe { ffi::GetTime() };
Ok((result).into_value_with(ruby))
},
"GetFPS" => { arity(args, 0)?;
let result = unsafe { ffi::GetFPS() };
Ok((result).into_value_with(ruby))
},
"SwapScreenBuffer" => { arity(args, 0)?;
let result = unsafe { ffi::SwapScreenBuffer() };
Ok(ruby.qnil().as_value())
},
"PollInputEvents" => { arity(args, 0)?;
let result = unsafe { ffi::PollInputEvents() };
Ok(ruby.qnil().as_value())
},
"WaitTime" => { arity(args, 1)?;
let result = unsafe { ffi::WaitTime(f64::try_convert(args[0])? as f64) };
Ok(ruby.qnil().as_value())
},
"SetRandomSeed" => { arity(args, 1)?;
let result = unsafe { ffi::SetRandomSeed(u32::try_from(u64::try_convert(args[0])?).map_err(|_| argument("unsigned int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"GetRandomValue" => { arity(args, 2)?;
let result = unsafe { ffi::GetRandomValue(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"LoadRandomSequence" => { arity(args, 3)?;
let result = unsafe { ffi::LoadRandomSequence(u32::try_from(u64::try_convert(args[0])?).map_err(|_| argument("unsigned int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(((result) as usize).into_value_with(ruby))
},
"UnloadRandomSequence" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadRandomSequence(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"TakeScreenshot" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::TakeScreenshot(text_0.pointer()) };
Ok(ruby.qnil().as_value())
},
"SetConfigFlags" => { arity(args, 1)?;
let result = unsafe { ffi::SetConfigFlags(u32::try_from(u64::try_convert(args[0])?).map_err(|_| argument("unsigned int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"OpenURL" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::OpenURL(text_0.pointer()) };
Ok(ruby.qnil().as_value())
},
"SetTraceLogLevel" => { arity(args, 1)?;
let result = unsafe { ffi::SetTraceLogLevel(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"TraceLog" => { arity(args, 2)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::TraceLog(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, c"%s".as_ptr(), text_1.pointer()) };
Ok(ruby.qnil().as_value())
},
"SetTraceLogCallback" => { arity(args, 1)?;
let result = unsafe { ffi::SetTraceLogCallback(unsafe { std::mem::transmute::<usize, ffi::TraceLogCallback>(address(args[0])?) }) };
Ok(ruby.qnil().as_value())
},
"MemAlloc" => { arity(args, 1)?;
let result = unsafe { ffi::MemAlloc(u32::try_from(u64::try_convert(args[0])?).map_err(|_| argument("unsigned int out of range"))?) };
Ok(((result) as usize).into_value_with(ruby))
},
"MemRealloc" => { arity(args, 2)?;
let result = unsafe { ffi::MemRealloc(address(args[0])? as _, u32::try_from(u64::try_convert(args[1])?).map_err(|_| argument("unsigned int out of range"))?) };
Ok(((result) as usize).into_value_with(ruby))
},
"MemFree" => { arity(args, 1)?;
let result = unsafe { ffi::MemFree(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"LoadFileData" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadFileData(text_0.pointer(), address(args[1])? as _) };
Ok(((result) as usize).into_value_with(ruby))
},
"UnloadFileData" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadFileData(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"SaveFileData" => { arity(args, 3)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::SaveFileData(text_0.pointer(), address(args[1])? as _, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"ExportDataAsCode" => { arity(args, 3)?;
let text_2 = string_argument(args[2])?;
let result = unsafe { ffi::ExportDataAsCode(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, text_2.pointer()) };
Ok((result).into_value_with(ruby))
},
"LoadFileText" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadFileText(text_0.pointer()) };
Ok(((result) as usize).into_value_with(ruby))
},
"UnloadFileText" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadFileText(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"SaveFileText" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::SaveFileText(text_0.pointer(), text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"SetLoadFileDataCallback" => { arity(args, 1)?;
let result = unsafe { ffi::SetLoadFileDataCallback(unsafe { std::mem::transmute::<usize, ffi::LoadFileDataCallback>(address(args[0])?) }) };
Ok(ruby.qnil().as_value())
},
"SetSaveFileDataCallback" => { arity(args, 1)?;
let result = unsafe { ffi::SetSaveFileDataCallback(unsafe { std::mem::transmute::<usize, ffi::SaveFileDataCallback>(address(args[0])?) }) };
Ok(ruby.qnil().as_value())
},
"SetLoadFileTextCallback" => { arity(args, 1)?;
let result = unsafe { ffi::SetLoadFileTextCallback(unsafe { std::mem::transmute::<usize, ffi::LoadFileTextCallback>(address(args[0])?) }) };
Ok(ruby.qnil().as_value())
},
"SetSaveFileTextCallback" => { arity(args, 1)?;
let result = unsafe { ffi::SetSaveFileTextCallback(unsafe { std::mem::transmute::<usize, ffi::SaveFileTextCallback>(address(args[0])?) }) };
Ok(ruby.qnil().as_value())
},
"FileRename" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::FileRename(text_0.pointer(), text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"FileRemove" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::FileRemove(text_0.pointer()) };
Ok((result).into_value_with(ruby))
},
"FileCopy" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::FileCopy(text_0.pointer(), text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"FileMove" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::FileMove(text_0.pointer(), text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"FileTextReplace" => { arity(args, 3)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let text_2 = string_argument(args[2])?;
let result = unsafe { ffi::FileTextReplace(text_0.pointer(), text_1.pointer(), text_2.pointer()) };
Ok((result).into_value_with(ruby))
},
"FileTextFindIndex" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::FileTextFindIndex(text_0.pointer(), text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"FileExists" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::FileExists(text_0.pointer()) };
Ok((result).into_value_with(ruby))
},
"DirectoryExists" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::DirectoryExists(text_0.pointer()) };
Ok((result).into_value_with(ruby))
},
"IsFileExtension" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::IsFileExtension(text_0.pointer(), text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"GetFileLength" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::GetFileLength(text_0.pointer()) };
Ok((result).into_value_with(ruby))
},
"GetFileModTime" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::GetFileModTime(text_0.pointer()) };
Ok((result).into_value_with(ruby))
},
"GetFileExtension" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::GetFileExtension(text_0.pointer()) };
Ok(if result.is_null() { ruby.qnil().as_value() } else { ruby.str_new(&unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy()).as_value() })
},
"GetFileName" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::GetFileName(text_0.pointer()) };
Ok(if result.is_null() { ruby.qnil().as_value() } else { ruby.str_new(&unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy()).as_value() })
},
"GetFileNameWithoutExt" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::GetFileNameWithoutExt(text_0.pointer()) };
Ok(if result.is_null() { ruby.qnil().as_value() } else { ruby.str_new(&unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy()).as_value() })
},
"GetDirectoryPath" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::GetDirectoryPath(text_0.pointer()) };
Ok(if result.is_null() { ruby.qnil().as_value() } else { ruby.str_new(&unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy()).as_value() })
},
"GetPrevDirectoryPath" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::GetPrevDirectoryPath(text_0.pointer()) };
Ok(if result.is_null() { ruby.qnil().as_value() } else { ruby.str_new(&unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy()).as_value() })
},
"GetWorkingDirectory" => { arity(args, 0)?;
let result = unsafe { ffi::GetWorkingDirectory() };
Ok(if result.is_null() { ruby.qnil().as_value() } else { ruby.str_new(&unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy()).as_value() })
},
"GetApplicationDirectory" => { arity(args, 0)?;
let result = unsafe { ffi::GetApplicationDirectory() };
Ok(if result.is_null() { ruby.qnil().as_value() } else { ruby.str_new(&unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy()).as_value() })
},
"MakeDirectory" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::MakeDirectory(text_0.pointer()) };
Ok((result).into_value_with(ruby))
},
"ChangeDirectory" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::ChangeDirectory(text_0.pointer()) };
Ok((result).into_value_with(ruby))
},
"IsPathFile" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::IsPathFile(text_0.pointer()) };
Ok((result).into_value_with(ruby))
},
"IsFileNameValid" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::IsFileNameValid(text_0.pointer()) };
Ok((result).into_value_with(ruby))
},
"LoadDirectoryFiles" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadDirectoryFiles(text_0.pointer()) };
Ok(write_FilePathList(ruby, result)?)
},
"LoadDirectoryFilesEx" => { arity(args, 3)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::LoadDirectoryFilesEx(text_0.pointer(), text_1.pointer(), bool::try_convert(args[2])?) };
Ok(write_FilePathList(ruby, result)?)
},
"UnloadDirectoryFiles" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadDirectoryFiles(read_FilePathList(args[0])?) };
Ok(ruby.qnil().as_value())
},
"IsFileDropped" => { arity(args, 0)?;
let result = unsafe { ffi::IsFileDropped() };
Ok((result).into_value_with(ruby))
},
"LoadDroppedFiles" => { arity(args, 0)?;
let result = unsafe { ffi::LoadDroppedFiles() };
Ok(write_FilePathList(ruby, result)?)
},
"UnloadDroppedFiles" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadDroppedFiles(read_FilePathList(args[0])?) };
Ok(ruby.qnil().as_value())
},
"GetDirectoryFileCount" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::GetDirectoryFileCount(text_0.pointer()) };
Ok((result).into_value_with(ruby))
},
"GetDirectoryFileCountEx" => { arity(args, 3)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::GetDirectoryFileCountEx(text_0.pointer(), text_1.pointer(), bool::try_convert(args[2])?) };
Ok((result).into_value_with(ruby))
},
"CompressData" => { arity(args, 3)?;
let result = unsafe { ffi::CompressData(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, address(args[2])? as _) };
Ok(((result) as usize).into_value_with(ruby))
},
"DecompressData" => { arity(args, 3)?;
let result = unsafe { ffi::DecompressData(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, address(args[2])? as _) };
Ok(((result) as usize).into_value_with(ruby))
},
"EncodeDataBase64" => { arity(args, 3)?;
let result = unsafe { ffi::EncodeDataBase64(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, address(args[2])? as _) };
Ok(((result) as usize).into_value_with(ruby))
},
"DecodeDataBase64" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::DecodeDataBase64(text_0.pointer(), address(args[1])? as _) };
Ok(((result) as usize).into_value_with(ruby))
},
"ComputeCRC32" => { arity(args, 2)?;
let result = unsafe { ffi::ComputeCRC32(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"ComputeMD5" => { arity(args, 2)?;
let result = unsafe { ffi::ComputeMD5(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(((result) as usize).into_value_with(ruby))
},
"ComputeSHA1" => { arity(args, 2)?;
let result = unsafe { ffi::ComputeSHA1(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(((result) as usize).into_value_with(ruby))
},
"ComputeSHA256" => { arity(args, 2)?;
let result = unsafe { ffi::ComputeSHA256(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(((result) as usize).into_value_with(ruby))
},
"LoadAutomationEventList" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadAutomationEventList(text_0.pointer()) };
Ok(write_AutomationEventList(ruby, result)?)
},
"UnloadAutomationEventList" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadAutomationEventList(read_AutomationEventList(args[0])?) };
Ok(ruby.qnil().as_value())
},
"ExportAutomationEventList" => { arity(args, 2)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::ExportAutomationEventList(read_AutomationEventList(args[0])?, text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"SetAutomationEventList" => { arity(args, 1)?;
let result = unsafe { ffi::SetAutomationEventList(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"SetAutomationEventBaseFrame" => { arity(args, 1)?;
let result = unsafe { ffi::SetAutomationEventBaseFrame(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"StartAutomationEventRecording" => { arity(args, 0)?;
let result = unsafe { ffi::StartAutomationEventRecording() };
Ok(ruby.qnil().as_value())
},
"StopAutomationEventRecording" => { arity(args, 0)?;
let result = unsafe { ffi::StopAutomationEventRecording() };
Ok(ruby.qnil().as_value())
},
"PlayAutomationEvent" => { arity(args, 1)?;
let result = unsafe { ffi::PlayAutomationEvent(read_AutomationEvent(args[0])?) };
Ok(ruby.qnil().as_value())
},
"IsKeyPressed" => { arity(args, 1)?;
let result = unsafe { ffi::IsKeyPressed(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"IsKeyPressedRepeat" => { arity(args, 1)?;
let result = unsafe { ffi::IsKeyPressedRepeat(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"IsKeyDown" => { arity(args, 1)?;
let result = unsafe { ffi::IsKeyDown(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"IsKeyReleased" => { arity(args, 1)?;
let result = unsafe { ffi::IsKeyReleased(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"IsKeyUp" => { arity(args, 1)?;
let result = unsafe { ffi::IsKeyUp(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"GetKeyPressed" => { arity(args, 0)?;
let result = unsafe { ffi::GetKeyPressed() };
Ok((result).into_value_with(ruby))
},
"GetCharPressed" => { arity(args, 0)?;
let result = unsafe { ffi::GetCharPressed() };
Ok((result).into_value_with(ruby))
},
"GetKeyName" => { arity(args, 1)?;
let result = unsafe { ffi::GetKeyName(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok(if result.is_null() { ruby.qnil().as_value() } else { ruby.str_new(&unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy()).as_value() })
},
"SetExitKey" => { arity(args, 1)?;
let result = unsafe { ffi::SetExitKey(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"IsGamepadAvailable" => { arity(args, 1)?;
let result = unsafe { ffi::IsGamepadAvailable(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"GetGamepadName" => { arity(args, 1)?;
let result = unsafe { ffi::GetGamepadName(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok(if result.is_null() { ruby.qnil().as_value() } else { ruby.str_new(&unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy()).as_value() })
},
"IsGamepadButtonPressed" => { arity(args, 2)?;
let result = unsafe { ffi::IsGamepadButtonPressed(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"IsGamepadButtonDown" => { arity(args, 2)?;
let result = unsafe { ffi::IsGamepadButtonDown(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"IsGamepadButtonReleased" => { arity(args, 2)?;
let result = unsafe { ffi::IsGamepadButtonReleased(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"IsGamepadButtonUp" => { arity(args, 2)?;
let result = unsafe { ffi::IsGamepadButtonUp(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"GetGamepadButtonPressed" => { arity(args, 0)?;
let result = unsafe { ffi::GetGamepadButtonPressed() };
Ok((result).into_value_with(ruby))
},
"GetGamepadAxisCount" => { arity(args, 1)?;
let result = unsafe { ffi::GetGamepadAxisCount(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"GetGamepadAxisMovement" => { arity(args, 2)?;
let result = unsafe { ffi::GetGamepadAxisMovement(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"SetGamepadMappings" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::SetGamepadMappings(text_0.pointer()) };
Ok((result).into_value_with(ruby))
},
"SetGamepadVibration" => { arity(args, 4)?;
let result = unsafe { ffi::SetGamepadVibration(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[1])? as f32, f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32) };
Ok(ruby.qnil().as_value())
},
"IsMouseButtonPressed" => { arity(args, 1)?;
let result = unsafe { ffi::IsMouseButtonPressed(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"IsMouseButtonDown" => { arity(args, 1)?;
let result = unsafe { ffi::IsMouseButtonDown(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"IsMouseButtonReleased" => { arity(args, 1)?;
let result = unsafe { ffi::IsMouseButtonReleased(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"IsMouseButtonUp" => { arity(args, 1)?;
let result = unsafe { ffi::IsMouseButtonUp(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"GetMouseX" => { arity(args, 0)?;
let result = unsafe { ffi::GetMouseX() };
Ok((result).into_value_with(ruby))
},
"GetMouseY" => { arity(args, 0)?;
let result = unsafe { ffi::GetMouseY() };
Ok((result).into_value_with(ruby))
},
"GetMousePosition" => { arity(args, 0)?;
let result = unsafe { ffi::GetMousePosition() };
Ok(write_Vector2(ruby, result)?)
},
"GetMouseDelta" => { arity(args, 0)?;
let result = unsafe { ffi::GetMouseDelta() };
Ok(write_Vector2(ruby, result)?)
},
"SetMousePosition" => { arity(args, 2)?;
let result = unsafe { ffi::SetMousePosition(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"SetMouseOffset" => { arity(args, 2)?;
let result = unsafe { ffi::SetMouseOffset(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"SetMouseScale" => { arity(args, 2)?;
let result = unsafe { ffi::SetMouseScale(f64::try_convert(args[0])? as f32, f64::try_convert(args[1])? as f32) };
Ok(ruby.qnil().as_value())
},
"GetMouseWheelMove" => { arity(args, 0)?;
let result = unsafe { ffi::GetMouseWheelMove() };
Ok((result).into_value_with(ruby))
},
"GetMouseWheelMoveV" => { arity(args, 0)?;
let result = unsafe { ffi::GetMouseWheelMoveV() };
Ok(write_Vector2(ruby, result)?)
},
"SetMouseCursor" => { arity(args, 1)?;
let result = unsafe { ffi::SetMouseCursor(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"GetTouchX" => { arity(args, 0)?;
let result = unsafe { ffi::GetTouchX() };
Ok((result).into_value_with(ruby))
},
"GetTouchY" => { arity(args, 0)?;
let result = unsafe { ffi::GetTouchY() };
Ok((result).into_value_with(ruby))
},
"GetTouchPosition" => { arity(args, 1)?;
let result = unsafe { ffi::GetTouchPosition(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Vector2(ruby, result)?)
},
"GetTouchPointId" => { arity(args, 1)?;
let result = unsafe { ffi::GetTouchPointId(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"GetTouchPointCount" => { arity(args, 0)?;
let result = unsafe { ffi::GetTouchPointCount() };
Ok((result).into_value_with(ruby))
},
"SetGesturesEnabled" => { arity(args, 1)?;
let result = unsafe { ffi::SetGesturesEnabled(u32::try_from(u64::try_convert(args[0])?).map_err(|_| argument("unsigned int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"IsGestureDetected" => { arity(args, 1)?;
let result = unsafe { ffi::IsGestureDetected(u32::try_from(u64::try_convert(args[0])?).map_err(|_| argument("unsigned int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"GetGestureDetected" => { arity(args, 0)?;
let result = unsafe { ffi::GetGestureDetected() };
Ok((result).into_value_with(ruby))
},
"GetGestureHoldDuration" => { arity(args, 0)?;
let result = unsafe { ffi::GetGestureHoldDuration() };
Ok((result).into_value_with(ruby))
},
"GetGestureDragVector" => { arity(args, 0)?;
let result = unsafe { ffi::GetGestureDragVector() };
Ok(write_Vector2(ruby, result)?)
},
"GetGestureDragAngle" => { arity(args, 0)?;
let result = unsafe { ffi::GetGestureDragAngle() };
Ok((result).into_value_with(ruby))
},
"GetGesturePinchVector" => { arity(args, 0)?;
let result = unsafe { ffi::GetGesturePinchVector() };
Ok(write_Vector2(ruby, result)?)
},
"GetGesturePinchAngle" => { arity(args, 0)?;
let result = unsafe { ffi::GetGesturePinchAngle() };
Ok((result).into_value_with(ruby))
},
"UpdateCamera" => { arity(args, 2)?;
let result = unsafe { ffi::UpdateCamera(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"UpdateCameraPro" => { arity(args, 4)?;
let result = unsafe { ffi::UpdateCameraPro(address(args[0])? as _, read_Vector3(args[1])?, read_Vector3(args[2])?, f64::try_convert(args[3])? as f32) };
Ok(ruby.qnil().as_value())
},
"SetShapesTexture" => { arity(args, 2)?;
let result = unsafe { ffi::SetShapesTexture(read_Texture(args[0])?, read_Rectangle(args[1])?) };
Ok(ruby.qnil().as_value())
},
"GetShapesTexture" => { arity(args, 0)?;
let result = unsafe { ffi::GetShapesTexture() };
Ok(write_Texture(ruby, result)?)
},
"GetShapesTextureRectangle" => { arity(args, 0)?;
let result = unsafe { ffi::GetShapesTextureRectangle() };
Ok(write_Rectangle(ruby, result)?)
},
"DrawPixel" => { arity(args, 3)?;
let result = unsafe { ffi::DrawPixel(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"DrawPixelV" => { arity(args, 2)?;
let result = unsafe { ffi::DrawPixelV(read_Vector2(args[0])?, read_Color(args[1])?) };
Ok(ruby.qnil().as_value())
},
"DrawLine" => { arity(args, 5)?;
let result = unsafe { ffi::DrawLine(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawLineV" => { arity(args, 3)?;
let result = unsafe { ffi::DrawLineV(read_Vector2(args[0])?, read_Vector2(args[1])?, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"DrawLineEx" => { arity(args, 4)?;
let result = unsafe { ffi::DrawLineEx(read_Vector2(args[0])?, read_Vector2(args[1])?, f64::try_convert(args[2])? as f32, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawLineStrip" => { arity(args, 3)?;
let result = unsafe { ffi::DrawLineStrip(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"DrawLineBezier" => { arity(args, 4)?;
let result = unsafe { ffi::DrawLineBezier(read_Vector2(args[0])?, read_Vector2(args[1])?, f64::try_convert(args[2])? as f32, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawLineDashed" => { arity(args, 5)?;
let result = unsafe { ffi::DrawLineDashed(read_Vector2(args[0])?, read_Vector2(args[1])?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawCircle" => { arity(args, 4)?;
let result = unsafe { ffi::DrawCircle(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[2])? as f32, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawCircleV" => { arity(args, 3)?;
let result = unsafe { ffi::DrawCircleV(read_Vector2(args[0])?, f64::try_convert(args[1])? as f32, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"DrawCircleGradient" => { arity(args, 4)?;
let result = unsafe { ffi::DrawCircleGradient(read_Vector2(args[0])?, f64::try_convert(args[1])? as f32, read_Color(args[2])?, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawCircleSector" => { arity(args, 6)?;
let result = unsafe { ffi::DrawCircleSector(read_Vector2(args[0])?, f64::try_convert(args[1])? as f32, f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawCircleSectorLines" => { arity(args, 6)?;
let result = unsafe { ffi::DrawCircleSectorLines(read_Vector2(args[0])?, f64::try_convert(args[1])? as f32, f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawCircleLines" => { arity(args, 4)?;
let result = unsafe { ffi::DrawCircleLines(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[2])? as f32, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawCircleLinesV" => { arity(args, 3)?;
let result = unsafe { ffi::DrawCircleLinesV(read_Vector2(args[0])?, f64::try_convert(args[1])? as f32, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"DrawEllipse" => { arity(args, 5)?;
let result = unsafe { ffi::DrawEllipse(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawEllipseV" => { arity(args, 4)?;
let result = unsafe { ffi::DrawEllipseV(read_Vector2(args[0])?, f64::try_convert(args[1])? as f32, f64::try_convert(args[2])? as f32, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawEllipseLines" => { arity(args, 5)?;
let result = unsafe { ffi::DrawEllipseLines(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawEllipseLinesV" => { arity(args, 4)?;
let result = unsafe { ffi::DrawEllipseLinesV(read_Vector2(args[0])?, f64::try_convert(args[1])? as f32, f64::try_convert(args[2])? as f32, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawRing" => { arity(args, 7)?;
let result = unsafe { ffi::DrawRing(read_Vector2(args[0])?, f64::try_convert(args[1])? as f32, f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32, f64::try_convert(args[4])? as f32, i32::try_from(i64::try_convert(args[5])?).map_err(|_| argument("int out of range"))?, read_Color(args[6])?) };
Ok(ruby.qnil().as_value())
},
"DrawRingLines" => { arity(args, 7)?;
let result = unsafe { ffi::DrawRingLines(read_Vector2(args[0])?, f64::try_convert(args[1])? as f32, f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32, f64::try_convert(args[4])? as f32, i32::try_from(i64::try_convert(args[5])?).map_err(|_| argument("int out of range"))?, read_Color(args[6])?) };
Ok(ruby.qnil().as_value())
},
"DrawRectangle" => { arity(args, 5)?;
let result = unsafe { ffi::DrawRectangle(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawRectangleV" => { arity(args, 3)?;
let result = unsafe { ffi::DrawRectangleV(read_Vector2(args[0])?, read_Vector2(args[1])?, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"DrawRectangleRec" => { arity(args, 2)?;
let result = unsafe { ffi::DrawRectangleRec(read_Rectangle(args[0])?, read_Color(args[1])?) };
Ok(ruby.qnil().as_value())
},
"DrawRectanglePro" => { arity(args, 4)?;
let result = unsafe { ffi::DrawRectanglePro(read_Rectangle(args[0])?, read_Vector2(args[1])?, f64::try_convert(args[2])? as f32, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawRectangleGradientV" => { arity(args, 6)?;
let result = unsafe { ffi::DrawRectangleGradientV(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, read_Color(args[4])?, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawRectangleGradientH" => { arity(args, 6)?;
let result = unsafe { ffi::DrawRectangleGradientH(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, read_Color(args[4])?, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawRectangleGradientEx" => { arity(args, 5)?;
let result = unsafe { ffi::DrawRectangleGradientEx(read_Rectangle(args[0])?, read_Color(args[1])?, read_Color(args[2])?, read_Color(args[3])?, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawRectangleLines" => { arity(args, 5)?;
let result = unsafe { ffi::DrawRectangleLines(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawRectangleLinesEx" => { arity(args, 3)?;
let result = unsafe { ffi::DrawRectangleLinesEx(read_Rectangle(args[0])?, f64::try_convert(args[1])? as f32, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"DrawRectangleRounded" => { arity(args, 4)?;
let result = unsafe { ffi::DrawRectangleRounded(read_Rectangle(args[0])?, f64::try_convert(args[1])? as f32, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawRectangleRoundedLines" => { arity(args, 4)?;
let result = unsafe { ffi::DrawRectangleRoundedLines(read_Rectangle(args[0])?, f64::try_convert(args[1])? as f32, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawRectangleRoundedLinesEx" => { arity(args, 5)?;
let result = unsafe { ffi::DrawRectangleRoundedLinesEx(read_Rectangle(args[0])?, f64::try_convert(args[1])? as f32, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[3])? as f32, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawTriangle" => { arity(args, 4)?;
let result = unsafe { ffi::DrawTriangle(read_Vector2(args[0])?, read_Vector2(args[1])?, read_Vector2(args[2])?, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawTriangleLines" => { arity(args, 4)?;
let result = unsafe { ffi::DrawTriangleLines(read_Vector2(args[0])?, read_Vector2(args[1])?, read_Vector2(args[2])?, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawTriangleFan" => { arity(args, 3)?;
let result = unsafe { ffi::DrawTriangleFan(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"DrawTriangleStrip" => { arity(args, 3)?;
let result = unsafe { ffi::DrawTriangleStrip(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"DrawPoly" => { arity(args, 5)?;
let result = unsafe { ffi::DrawPoly(read_Vector2(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawPolyLines" => { arity(args, 5)?;
let result = unsafe { ffi::DrawPolyLines(read_Vector2(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawPolyLinesEx" => { arity(args, 6)?;
let result = unsafe { ffi::DrawPolyLinesEx(read_Vector2(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32, f64::try_convert(args[4])? as f32, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawSplineLinear" => { arity(args, 4)?;
let result = unsafe { ffi::DrawSplineLinear(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[2])? as f32, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawSplineBasis" => { arity(args, 4)?;
let result = unsafe { ffi::DrawSplineBasis(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[2])? as f32, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawSplineCatmullRom" => { arity(args, 4)?;
let result = unsafe { ffi::DrawSplineCatmullRom(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[2])? as f32, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawSplineBezierQuadratic" => { arity(args, 4)?;
let result = unsafe { ffi::DrawSplineBezierQuadratic(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[2])? as f32, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawSplineBezierCubic" => { arity(args, 4)?;
let result = unsafe { ffi::DrawSplineBezierCubic(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[2])? as f32, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawSplineSegmentLinear" => { arity(args, 4)?;
let result = unsafe { ffi::DrawSplineSegmentLinear(read_Vector2(args[0])?, read_Vector2(args[1])?, f64::try_convert(args[2])? as f32, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawSplineSegmentBasis" => { arity(args, 6)?;
let result = unsafe { ffi::DrawSplineSegmentBasis(read_Vector2(args[0])?, read_Vector2(args[1])?, read_Vector2(args[2])?, read_Vector2(args[3])?, f64::try_convert(args[4])? as f32, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawSplineSegmentCatmullRom" => { arity(args, 6)?;
let result = unsafe { ffi::DrawSplineSegmentCatmullRom(read_Vector2(args[0])?, read_Vector2(args[1])?, read_Vector2(args[2])?, read_Vector2(args[3])?, f64::try_convert(args[4])? as f32, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawSplineSegmentBezierQuadratic" => { arity(args, 5)?;
let result = unsafe { ffi::DrawSplineSegmentBezierQuadratic(read_Vector2(args[0])?, read_Vector2(args[1])?, read_Vector2(args[2])?, f64::try_convert(args[3])? as f32, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawSplineSegmentBezierCubic" => { arity(args, 6)?;
let result = unsafe { ffi::DrawSplineSegmentBezierCubic(read_Vector2(args[0])?, read_Vector2(args[1])?, read_Vector2(args[2])?, read_Vector2(args[3])?, f64::try_convert(args[4])? as f32, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"GetSplinePointLinear" => { arity(args, 3)?;
let result = unsafe { ffi::GetSplinePointLinear(read_Vector2(args[0])?, read_Vector2(args[1])?, f64::try_convert(args[2])? as f32) };
Ok(write_Vector2(ruby, result)?)
},
"GetSplinePointBasis" => { arity(args, 5)?;
let result = unsafe { ffi::GetSplinePointBasis(read_Vector2(args[0])?, read_Vector2(args[1])?, read_Vector2(args[2])?, read_Vector2(args[3])?, f64::try_convert(args[4])? as f32) };
Ok(write_Vector2(ruby, result)?)
},
"GetSplinePointCatmullRom" => { arity(args, 5)?;
let result = unsafe { ffi::GetSplinePointCatmullRom(read_Vector2(args[0])?, read_Vector2(args[1])?, read_Vector2(args[2])?, read_Vector2(args[3])?, f64::try_convert(args[4])? as f32) };
Ok(write_Vector2(ruby, result)?)
},
"GetSplinePointBezierQuad" => { arity(args, 4)?;
let result = unsafe { ffi::GetSplinePointBezierQuad(read_Vector2(args[0])?, read_Vector2(args[1])?, read_Vector2(args[2])?, f64::try_convert(args[3])? as f32) };
Ok(write_Vector2(ruby, result)?)
},
"GetSplinePointBezierCubic" => { arity(args, 5)?;
let result = unsafe { ffi::GetSplinePointBezierCubic(read_Vector2(args[0])?, read_Vector2(args[1])?, read_Vector2(args[2])?, read_Vector2(args[3])?, f64::try_convert(args[4])? as f32) };
Ok(write_Vector2(ruby, result)?)
},
"CheckCollisionRecs" => { arity(args, 2)?;
let result = unsafe { ffi::CheckCollisionRecs(read_Rectangle(args[0])?, read_Rectangle(args[1])?) };
Ok((result).into_value_with(ruby))
},
"CheckCollisionCircles" => { arity(args, 4)?;
let result = unsafe { ffi::CheckCollisionCircles(read_Vector2(args[0])?, f64::try_convert(args[1])? as f32, read_Vector2(args[2])?, f64::try_convert(args[3])? as f32) };
Ok((result).into_value_with(ruby))
},
"CheckCollisionCircleRec" => { arity(args, 3)?;
let result = unsafe { ffi::CheckCollisionCircleRec(read_Vector2(args[0])?, f64::try_convert(args[1])? as f32, read_Rectangle(args[2])?) };
Ok((result).into_value_with(ruby))
},
"CheckCollisionCircleLine" => { arity(args, 4)?;
let result = unsafe { ffi::CheckCollisionCircleLine(read_Vector2(args[0])?, f64::try_convert(args[1])? as f32, read_Vector2(args[2])?, read_Vector2(args[3])?) };
Ok((result).into_value_with(ruby))
},
"CheckCollisionPointRec" => { arity(args, 2)?;
let result = unsafe { ffi::CheckCollisionPointRec(read_Vector2(args[0])?, read_Rectangle(args[1])?) };
Ok((result).into_value_with(ruby))
},
"CheckCollisionPointCircle" => { arity(args, 3)?;
let result = unsafe { ffi::CheckCollisionPointCircle(read_Vector2(args[0])?, read_Vector2(args[1])?, f64::try_convert(args[2])? as f32) };
Ok((result).into_value_with(ruby))
},
"CheckCollisionPointTriangle" => { arity(args, 4)?;
let result = unsafe { ffi::CheckCollisionPointTriangle(read_Vector2(args[0])?, read_Vector2(args[1])?, read_Vector2(args[2])?, read_Vector2(args[3])?) };
Ok((result).into_value_with(ruby))
},
"CheckCollisionPointLine" => { arity(args, 4)?;
let result = unsafe { ffi::CheckCollisionPointLine(read_Vector2(args[0])?, read_Vector2(args[1])?, read_Vector2(args[2])?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"CheckCollisionPointPoly" => { arity(args, 3)?;
let result = unsafe { ffi::CheckCollisionPointPoly(read_Vector2(args[0])?, address(args[1])? as _, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"CheckCollisionLines" => { arity(args, 5)?;
let result = unsafe { ffi::CheckCollisionLines(read_Vector2(args[0])?, read_Vector2(args[1])?, read_Vector2(args[2])?, read_Vector2(args[3])?, address(args[4])? as _) };
Ok((result).into_value_with(ruby))
},
"GetCollisionRec" => { arity(args, 2)?;
let result = unsafe { ffi::GetCollisionRec(read_Rectangle(args[0])?, read_Rectangle(args[1])?) };
Ok(write_Rectangle(ruby, result)?)
},
"LoadImage" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadImage(text_0.pointer()) };
Ok(write_Image(ruby, result)?)
},
"LoadImageRaw" => { arity(args, 5)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadImageRaw(text_0.pointer(), i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Image(ruby, result)?)
},
"LoadImageAnim" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadImageAnim(text_0.pointer(), address(args[1])? as _) };
Ok(write_Image(ruby, result)?)
},
"LoadImageAnimFromMemory" => { arity(args, 4)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadImageAnimFromMemory(text_0.pointer(), address(args[1])? as _, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, address(args[3])? as _) };
Ok(write_Image(ruby, result)?)
},
"LoadImageFromMemory" => { arity(args, 3)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadImageFromMemory(text_0.pointer(), address(args[1])? as _, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Image(ruby, result)?)
},
"LoadImageFromTexture" => { arity(args, 1)?;
let result = unsafe { ffi::LoadImageFromTexture(read_Texture(args[0])?) };
Ok(write_Image(ruby, result)?)
},
"LoadImageFromScreen" => { arity(args, 0)?;
let result = unsafe { ffi::LoadImageFromScreen() };
Ok(write_Image(ruby, result)?)
},
"IsImageValid" => { arity(args, 1)?;
let result = unsafe { ffi::IsImageValid(read_Image(args[0])?) };
Ok((result).into_value_with(ruby))
},
"UnloadImage" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadImage(read_Image(args[0])?) };
Ok(ruby.qnil().as_value())
},
"ExportImage" => { arity(args, 2)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::ExportImage(read_Image(args[0])?, text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"ExportImageToMemory" => { arity(args, 3)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::ExportImageToMemory(read_Image(args[0])?, text_1.pointer(), address(args[2])? as _) };
Ok(((result) as usize).into_value_with(ruby))
},
"ExportImageAsCode" => { arity(args, 2)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::ExportImageAsCode(read_Image(args[0])?, text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"GenImageColor" => { arity(args, 3)?;
let result = unsafe { ffi::GenImageColor(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, read_Color(args[2])?) };
Ok(write_Image(ruby, result)?)
},
"GenImageGradientLinear" => { arity(args, 5)?;
let result = unsafe { ffi::GenImageGradientLinear(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, read_Color(args[3])?, read_Color(args[4])?) };
Ok(write_Image(ruby, result)?)
},
"GenImageGradientRadial" => { arity(args, 5)?;
let result = unsafe { ffi::GenImageGradientRadial(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[2])? as f32, read_Color(args[3])?, read_Color(args[4])?) };
Ok(write_Image(ruby, result)?)
},
"GenImageGradientSquare" => { arity(args, 5)?;
let result = unsafe { ffi::GenImageGradientSquare(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[2])? as f32, read_Color(args[3])?, read_Color(args[4])?) };
Ok(write_Image(ruby, result)?)
},
"GenImageChecked" => { arity(args, 6)?;
let result = unsafe { ffi::GenImageChecked(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, read_Color(args[4])?, read_Color(args[5])?) };
Ok(write_Image(ruby, result)?)
},
"GenImageWhiteNoise" => { arity(args, 3)?;
let result = unsafe { ffi::GenImageWhiteNoise(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[2])? as f32) };
Ok(write_Image(ruby, result)?)
},
"GenImagePerlinNoise" => { arity(args, 5)?;
let result = unsafe { ffi::GenImagePerlinNoise(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[4])? as f32) };
Ok(write_Image(ruby, result)?)
},
"GenImageCellular" => { arity(args, 3)?;
let result = unsafe { ffi::GenImageCellular(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Image(ruby, result)?)
},
"GenImageText" => { arity(args, 3)?;
let text_2 = string_argument(args[2])?;
let result = unsafe { ffi::GenImageText(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, text_2.pointer()) };
Ok(write_Image(ruby, result)?)
},
"ImageCopy" => { arity(args, 1)?;
let result = unsafe { ffi::ImageCopy(read_Image(args[0])?) };
Ok(write_Image(ruby, result)?)
},
"ImageFromImage" => { arity(args, 2)?;
let result = unsafe { ffi::ImageFromImage(read_Image(args[0])?, read_Rectangle(args[1])?) };
Ok(write_Image(ruby, result)?)
},
"ImageFromChannel" => { arity(args, 2)?;
let result = unsafe { ffi::ImageFromChannel(read_Image(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Image(ruby, result)?)
},
"ImageText" => { arity(args, 3)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::ImageText(text_0.pointer(), i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, read_Color(args[2])?) };
Ok(write_Image(ruby, result)?)
},
"ImageTextEx" => { arity(args, 5)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::ImageTextEx(read_Font(args[0])?, text_1.pointer(), f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32, read_Color(args[4])?) };
Ok(write_Image(ruby, result)?)
},
"ImageFormat" => { arity(args, 2)?;
let result = unsafe { ffi::ImageFormat(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"ImageToPOT" => { arity(args, 2)?;
let result = unsafe { ffi::ImageToPOT(address(args[0])? as _, read_Color(args[1])?) };
Ok(ruby.qnil().as_value())
},
"ImageCrop" => { arity(args, 2)?;
let result = unsafe { ffi::ImageCrop(address(args[0])? as _, read_Rectangle(args[1])?) };
Ok(ruby.qnil().as_value())
},
"ImageAlphaCrop" => { arity(args, 2)?;
let result = unsafe { ffi::ImageAlphaCrop(address(args[0])? as _, f64::try_convert(args[1])? as f32) };
Ok(ruby.qnil().as_value())
},
"ImageAlphaClear" => { arity(args, 3)?;
let result = unsafe { ffi::ImageAlphaClear(address(args[0])? as _, read_Color(args[1])?, f64::try_convert(args[2])? as f32) };
Ok(ruby.qnil().as_value())
},
"ImageAlphaMask" => { arity(args, 2)?;
let result = unsafe { ffi::ImageAlphaMask(address(args[0])? as _, read_Image(args[1])?) };
Ok(ruby.qnil().as_value())
},
"ImageAlphaPremultiply" => { arity(args, 1)?;
let result = unsafe { ffi::ImageAlphaPremultiply(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"ImageBlurGaussian" => { arity(args, 2)?;
let result = unsafe { ffi::ImageBlurGaussian(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"ImageKernelConvolution" => { arity(args, 3)?;
let result = unsafe { ffi::ImageKernelConvolution(address(args[0])? as _, address(args[1])? as _, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"ImageResize" => { arity(args, 3)?;
let result = unsafe { ffi::ImageResize(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"ImageResizeNN" => { arity(args, 3)?;
let result = unsafe { ffi::ImageResizeNN(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"ImageResizeCanvas" => { arity(args, 6)?;
let result = unsafe { ffi::ImageResizeCanvas(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"ImageMipmaps" => { arity(args, 1)?;
let result = unsafe { ffi::ImageMipmaps(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"ImageDither" => { arity(args, 5)?;
let result = unsafe { ffi::ImageDither(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"ImageFlipVertical" => { arity(args, 1)?;
let result = unsafe { ffi::ImageFlipVertical(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"ImageFlipHorizontal" => { arity(args, 1)?;
let result = unsafe { ffi::ImageFlipHorizontal(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"ImageRotate" => { arity(args, 2)?;
let result = unsafe { ffi::ImageRotate(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"ImageRotateCW" => { arity(args, 1)?;
let result = unsafe { ffi::ImageRotateCW(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"ImageRotateCCW" => { arity(args, 1)?;
let result = unsafe { ffi::ImageRotateCCW(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"ImageColorTint" => { arity(args, 2)?;
let result = unsafe { ffi::ImageColorTint(address(args[0])? as _, read_Color(args[1])?) };
Ok(ruby.qnil().as_value())
},
"ImageColorInvert" => { arity(args, 1)?;
let result = unsafe { ffi::ImageColorInvert(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"ImageColorGrayscale" => { arity(args, 1)?;
let result = unsafe { ffi::ImageColorGrayscale(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"ImageColorContrast" => { arity(args, 2)?;
let result = unsafe { ffi::ImageColorContrast(address(args[0])? as _, f64::try_convert(args[1])? as f32) };
Ok(ruby.qnil().as_value())
},
"ImageColorBrightness" => { arity(args, 2)?;
let result = unsafe { ffi::ImageColorBrightness(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"ImageColorReplace" => { arity(args, 3)?;
let result = unsafe { ffi::ImageColorReplace(address(args[0])? as _, read_Color(args[1])?, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"LoadImageColors" => { arity(args, 1)?;
let result = unsafe { ffi::LoadImageColors(read_Image(args[0])?) };
Ok(((result) as usize).into_value_with(ruby))
},
"LoadImagePalette" => { arity(args, 3)?;
let result = unsafe { ffi::LoadImagePalette(read_Image(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, address(args[2])? as _) };
Ok(((result) as usize).into_value_with(ruby))
},
"UnloadImageColors" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadImageColors(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"UnloadImagePalette" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadImagePalette(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"GetImageAlphaBorder" => { arity(args, 2)?;
let result = unsafe { ffi::GetImageAlphaBorder(read_Image(args[0])?, f64::try_convert(args[1])? as f32) };
Ok(write_Rectangle(ruby, result)?)
},
"GetImageColor" => { arity(args, 3)?;
let result = unsafe { ffi::GetImageColor(read_Image(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Color(ruby, result)?)
},
"ImageClearBackground" => { arity(args, 2)?;
let result = unsafe { ffi::ImageClearBackground(address(args[0])? as _, read_Color(args[1])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawPixel" => { arity(args, 4)?;
let result = unsafe { ffi::ImageDrawPixel(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawPixelV" => { arity(args, 3)?;
let result = unsafe { ffi::ImageDrawPixelV(address(args[0])? as _, read_Vector2(args[1])?, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawLine" => { arity(args, 6)?;
let result = unsafe { ffi::ImageDrawLine(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawLineV" => { arity(args, 4)?;
let result = unsafe { ffi::ImageDrawLineV(address(args[0])? as _, read_Vector2(args[1])?, read_Vector2(args[2])?, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawLineEx" => { arity(args, 5)?;
let result = unsafe { ffi::ImageDrawLineEx(address(args[0])? as _, read_Vector2(args[1])?, read_Vector2(args[2])?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawCircle" => { arity(args, 5)?;
let result = unsafe { ffi::ImageDrawCircle(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawCircleV" => { arity(args, 4)?;
let result = unsafe { ffi::ImageDrawCircleV(address(args[0])? as _, read_Vector2(args[1])?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawCircleLines" => { arity(args, 5)?;
let result = unsafe { ffi::ImageDrawCircleLines(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawCircleLinesV" => { arity(args, 4)?;
let result = unsafe { ffi::ImageDrawCircleLinesV(address(args[0])? as _, read_Vector2(args[1])?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawRectangle" => { arity(args, 6)?;
let result = unsafe { ffi::ImageDrawRectangle(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawRectangleV" => { arity(args, 4)?;
let result = unsafe { ffi::ImageDrawRectangleV(address(args[0])? as _, read_Vector2(args[1])?, read_Vector2(args[2])?, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawRectangleRec" => { arity(args, 3)?;
let result = unsafe { ffi::ImageDrawRectangleRec(address(args[0])? as _, read_Rectangle(args[1])?, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawRectangleLines" => { arity(args, 4)?;
let result = unsafe { ffi::ImageDrawRectangleLines(address(args[0])? as _, read_Rectangle(args[1])?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawTriangle" => { arity(args, 5)?;
let result = unsafe { ffi::ImageDrawTriangle(address(args[0])? as _, read_Vector2(args[1])?, read_Vector2(args[2])?, read_Vector2(args[3])?, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawTriangleEx" => { arity(args, 7)?;
let result = unsafe { ffi::ImageDrawTriangleEx(address(args[0])? as _, read_Vector2(args[1])?, read_Vector2(args[2])?, read_Vector2(args[3])?, read_Color(args[4])?, read_Color(args[5])?, read_Color(args[6])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawTriangleLines" => { arity(args, 5)?;
let result = unsafe { ffi::ImageDrawTriangleLines(address(args[0])? as _, read_Vector2(args[1])?, read_Vector2(args[2])?, read_Vector2(args[3])?, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawTriangleFan" => { arity(args, 4)?;
let result = unsafe { ffi::ImageDrawTriangleFan(address(args[0])? as _, address(args[1])? as _, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawTriangleStrip" => { arity(args, 4)?;
let result = unsafe { ffi::ImageDrawTriangleStrip(address(args[0])? as _, address(args[1])? as _, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"ImageDraw" => { arity(args, 5)?;
let result = unsafe { ffi::ImageDraw(address(args[0])? as _, read_Image(args[1])?, read_Rectangle(args[2])?, read_Rectangle(args[3])?, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawText" => { arity(args, 6)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::ImageDrawText(address(args[0])? as _, text_1.pointer(), i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"ImageDrawTextEx" => { arity(args, 7)?;
let text_2 = string_argument(args[2])?;
let result = unsafe { ffi::ImageDrawTextEx(address(args[0])? as _, read_Font(args[1])?, text_2.pointer(), read_Vector2(args[3])?, f64::try_convert(args[4])? as f32, f64::try_convert(args[5])? as f32, read_Color(args[6])?) };
Ok(ruby.qnil().as_value())
},
"LoadTexture" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadTexture(text_0.pointer()) };
Ok(write_Texture(ruby, result)?)
},
"LoadTextureFromImage" => { arity(args, 1)?;
let result = unsafe { ffi::LoadTextureFromImage(read_Image(args[0])?) };
Ok(write_Texture(ruby, result)?)
},
"LoadTextureCubemap" => { arity(args, 2)?;
let result = unsafe { ffi::LoadTextureCubemap(read_Image(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Texture(ruby, result)?)
},
"LoadRenderTexture" => { arity(args, 2)?;
let result = unsafe { ffi::LoadRenderTexture(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(write_RenderTexture(ruby, result)?)
},
"IsTextureValid" => { arity(args, 1)?;
let result = unsafe { ffi::IsTextureValid(read_Texture(args[0])?) };
Ok((result).into_value_with(ruby))
},
"UnloadTexture" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadTexture(read_Texture(args[0])?) };
Ok(ruby.qnil().as_value())
},
"IsRenderTextureValid" => { arity(args, 1)?;
let result = unsafe { ffi::IsRenderTextureValid(read_RenderTexture(args[0])?) };
Ok((result).into_value_with(ruby))
},
"UnloadRenderTexture" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadRenderTexture(read_RenderTexture(args[0])?) };
Ok(ruby.qnil().as_value())
},
"UpdateTexture" => { arity(args, 2)?;
let result = unsafe { ffi::UpdateTexture(read_Texture(args[0])?, address(args[1])? as _) };
Ok(ruby.qnil().as_value())
},
"UpdateTextureRec" => { arity(args, 3)?;
let result = unsafe { ffi::UpdateTextureRec(read_Texture(args[0])?, read_Rectangle(args[1])?, address(args[2])? as _) };
Ok(ruby.qnil().as_value())
},
"GenTextureMipmaps" => { arity(args, 1)?;
let result = unsafe { ffi::GenTextureMipmaps(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"SetTextureFilter" => { arity(args, 2)?;
let result = unsafe { ffi::SetTextureFilter(read_Texture(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"SetTextureWrap" => { arity(args, 2)?;
let result = unsafe { ffi::SetTextureWrap(read_Texture(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"DrawTexture" => { arity(args, 4)?;
let result = unsafe { ffi::DrawTexture(read_Texture(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawTextureV" => { arity(args, 3)?;
let result = unsafe { ffi::DrawTextureV(read_Texture(args[0])?, read_Vector2(args[1])?, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"DrawTextureEx" => { arity(args, 5)?;
let result = unsafe { ffi::DrawTextureEx(read_Texture(args[0])?, read_Vector2(args[1])?, f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawTextureRec" => { arity(args, 4)?;
let result = unsafe { ffi::DrawTextureRec(read_Texture(args[0])?, read_Rectangle(args[1])?, read_Vector2(args[2])?, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawTexturePro" => { arity(args, 6)?;
let result = unsafe { ffi::DrawTexturePro(read_Texture(args[0])?, read_Rectangle(args[1])?, read_Rectangle(args[2])?, read_Vector2(args[3])?, f64::try_convert(args[4])? as f32, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawTextureNPatch" => { arity(args, 6)?;
let result = unsafe { ffi::DrawTextureNPatch(read_Texture(args[0])?, read_NPatchInfo(args[1])?, read_Rectangle(args[2])?, read_Vector2(args[3])?, f64::try_convert(args[4])? as f32, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"ColorIsEqual" => { arity(args, 2)?;
let result = unsafe { ffi::ColorIsEqual(read_Color(args[0])?, read_Color(args[1])?) };
Ok((result).into_value_with(ruby))
},
"Fade" => { arity(args, 2)?;
let result = unsafe { ffi::Fade(read_Color(args[0])?, f64::try_convert(args[1])? as f32) };
Ok(write_Color(ruby, result)?)
},
"ColorToInt" => { arity(args, 1)?;
let result = unsafe { ffi::ColorToInt(read_Color(args[0])?) };
Ok((result).into_value_with(ruby))
},
"ColorNormalize" => { arity(args, 1)?;
let result = unsafe { ffi::ColorNormalize(read_Color(args[0])?) };
Ok(write_Vector4(ruby, result)?)
},
"ColorFromNormalized" => { arity(args, 1)?;
let result = unsafe { ffi::ColorFromNormalized(read_Vector4(args[0])?) };
Ok(write_Color(ruby, result)?)
},
"ColorToHSV" => { arity(args, 1)?;
let result = unsafe { ffi::ColorToHSV(read_Color(args[0])?) };
Ok(write_Vector3(ruby, result)?)
},
"ColorFromHSV" => { arity(args, 3)?;
let result = unsafe { ffi::ColorFromHSV(f64::try_convert(args[0])? as f32, f64::try_convert(args[1])? as f32, f64::try_convert(args[2])? as f32) };
Ok(write_Color(ruby, result)?)
},
"ColorTint" => { arity(args, 2)?;
let result = unsafe { ffi::ColorTint(read_Color(args[0])?, read_Color(args[1])?) };
Ok(write_Color(ruby, result)?)
},
"ColorBrightness" => { arity(args, 2)?;
let result = unsafe { ffi::ColorBrightness(read_Color(args[0])?, f64::try_convert(args[1])? as f32) };
Ok(write_Color(ruby, result)?)
},
"ColorContrast" => { arity(args, 2)?;
let result = unsafe { ffi::ColorContrast(read_Color(args[0])?, f64::try_convert(args[1])? as f32) };
Ok(write_Color(ruby, result)?)
},
"ColorAlpha" => { arity(args, 2)?;
let result = unsafe { ffi::ColorAlpha(read_Color(args[0])?, f64::try_convert(args[1])? as f32) };
Ok(write_Color(ruby, result)?)
},
"ColorAlphaBlend" => { arity(args, 3)?;
let result = unsafe { ffi::ColorAlphaBlend(read_Color(args[0])?, read_Color(args[1])?, read_Color(args[2])?) };
Ok(write_Color(ruby, result)?)
},
"ColorLerp" => { arity(args, 3)?;
let result = unsafe { ffi::ColorLerp(read_Color(args[0])?, read_Color(args[1])?, f64::try_convert(args[2])? as f32) };
Ok(write_Color(ruby, result)?)
},
"GetColor" => { arity(args, 1)?;
let result = unsafe { ffi::GetColor(u32::try_from(u64::try_convert(args[0])?).map_err(|_| argument("unsigned int out of range"))?) };
Ok(write_Color(ruby, result)?)
},
"GetPixelColor" => { arity(args, 2)?;
let result = unsafe { ffi::GetPixelColor(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Color(ruby, result)?)
},
"SetPixelColor" => { arity(args, 3)?;
let result = unsafe { ffi::SetPixelColor(address(args[0])? as _, read_Color(args[1])?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"GetPixelDataSize" => { arity(args, 3)?;
let result = unsafe { ffi::GetPixelDataSize(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"GetFontDefault" => { arity(args, 0)?;
let result = unsafe { ffi::GetFontDefault() };
Ok(write_Font(ruby, result)?)
},
"LoadFont" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadFont(text_0.pointer()) };
Ok(write_Font(ruby, result)?)
},
"LoadFontEx" => { arity(args, 4)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadFontEx(text_0.pointer(), i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, address(args[2])? as _, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Font(ruby, result)?)
},
"LoadFontFromImage" => { arity(args, 3)?;
let result = unsafe { ffi::LoadFontFromImage(read_Image(args[0])?, read_Color(args[1])?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Font(ruby, result)?)
},
"LoadFontFromMemory" => { arity(args, 6)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadFontFromMemory(text_0.pointer(), address(args[1])? as _, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, address(args[4])? as _, i32::try_from(i64::try_convert(args[5])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Font(ruby, result)?)
},
"IsFontValid" => { arity(args, 1)?;
let result = unsafe { ffi::IsFontValid(read_Font(args[0])?) };
Ok((result).into_value_with(ruby))
},
"LoadFontData" => { arity(args, 7)?;
let result = unsafe { ffi::LoadFontData(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, address(args[3])? as _, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[5])?).map_err(|_| argument("int out of range"))?, address(args[6])? as _) };
Ok(((result) as usize).into_value_with(ruby))
},
"GenImageFontAtlas" => { arity(args, 6)?;
let result = unsafe { ffi::GenImageFontAtlas(address(args[0])? as _, address(args[1])? as _, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[5])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Image(ruby, result)?)
},
"UnloadFontData" => { arity(args, 2)?;
let result = unsafe { ffi::UnloadFontData(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"UnloadFont" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadFont(read_Font(args[0])?) };
Ok(ruby.qnil().as_value())
},
"ExportFontAsCode" => { arity(args, 2)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::ExportFontAsCode(read_Font(args[0])?, text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"DrawFPS" => { arity(args, 2)?;
let result = unsafe { ffi::DrawFPS(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"DrawText" => { arity(args, 5)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::DrawText(text_0.pointer(), i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawTextEx" => { arity(args, 6)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::DrawTextEx(read_Font(args[0])?, text_1.pointer(), read_Vector2(args[2])?, f64::try_convert(args[3])? as f32, f64::try_convert(args[4])? as f32, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawTextPro" => { arity(args, 8)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::DrawTextPro(read_Font(args[0])?, text_1.pointer(), read_Vector2(args[2])?, read_Vector2(args[3])?, f64::try_convert(args[4])? as f32, f64::try_convert(args[5])? as f32, f64::try_convert(args[6])? as f32, read_Color(args[7])?) };
Ok(ruby.qnil().as_value())
},
"DrawTextCodepoint" => { arity(args, 5)?;
let result = unsafe { ffi::DrawTextCodepoint(read_Font(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, read_Vector2(args[2])?, f64::try_convert(args[3])? as f32, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawTextCodepoints" => { arity(args, 7)?;
let result = unsafe { ffi::DrawTextCodepoints(read_Font(args[0])?, address(args[1])? as _, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, read_Vector2(args[3])?, f64::try_convert(args[4])? as f32, f64::try_convert(args[5])? as f32, read_Color(args[6])?) };
Ok(ruby.qnil().as_value())
},
"SetTextLineSpacing" => { arity(args, 1)?;
let result = unsafe { ffi::SetTextLineSpacing(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"MeasureText" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::MeasureText(text_0.pointer(), i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"MeasureTextEx" => { arity(args, 4)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::MeasureTextEx(read_Font(args[0])?, text_1.pointer(), f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32) };
Ok(write_Vector2(ruby, result)?)
},
"MeasureTextCodepoints" => { arity(args, 5)?;
let result = unsafe { ffi::MeasureTextCodepoints(read_Font(args[0])?, address(args[1])? as _, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[3])? as f32, f64::try_convert(args[4])? as f32) };
Ok(write_Vector2(ruby, result)?)
},
"GetGlyphIndex" => { arity(args, 2)?;
let result = unsafe { ffi::GetGlyphIndex(read_Font(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok((result).into_value_with(ruby))
},
"GetGlyphInfo" => { arity(args, 2)?;
let result = unsafe { ffi::GetGlyphInfo(read_Font(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(write_GlyphInfo(ruby, result)?)
},
"GetGlyphAtlasRec" => { arity(args, 2)?;
let result = unsafe { ffi::GetGlyphAtlasRec(read_Font(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Rectangle(ruby, result)?)
},
"LoadUTF8" => { arity(args, 2)?;
let result = unsafe { ffi::LoadUTF8(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(((result) as usize).into_value_with(ruby))
},
"UnloadUTF8" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadUTF8(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"LoadCodepoints" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadCodepoints(text_0.pointer(), address(args[1])? as _) };
Ok(((result) as usize).into_value_with(ruby))
},
"UnloadCodepoints" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadCodepoints(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"GetCodepointCount" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::GetCodepointCount(text_0.pointer()) };
Ok((result).into_value_with(ruby))
},
"GetCodepoint" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::GetCodepoint(text_0.pointer(), address(args[1])? as _) };
Ok((result).into_value_with(ruby))
},
"GetCodepointNext" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::GetCodepointNext(text_0.pointer(), address(args[1])? as _) };
Ok((result).into_value_with(ruby))
},
"GetCodepointPrevious" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::GetCodepointPrevious(text_0.pointer(), address(args[1])? as _) };
Ok((result).into_value_with(ruby))
},
"CodepointToUTF8" => { arity(args, 2)?;
let result = unsafe { ffi::CodepointToUTF8(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, address(args[1])? as _) };
Ok(if result.is_null() { ruby.qnil().as_value() } else { ruby.str_new(&unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy()).as_value() })
},
"LoadTextLines" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadTextLines(text_0.pointer(), address(args[1])? as _) };
Ok(((result) as usize).into_value_with(ruby))
},
"UnloadTextLines" => { arity(args, 2)?;
let result = unsafe { ffi::UnloadTextLines(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"TextCopy" => { arity(args, 2)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::TextCopy(address(args[0])? as _, text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"TextIsEqual" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::TextIsEqual(text_0.pointer(), text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"TextLength" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::TextLength(text_0.pointer()) };
Ok((result).into_value_with(ruby))
},
"TextFormat" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::TextFormat(c"%s".as_ptr(), text_0.pointer()) };
Ok(if result.is_null() { ruby.qnil().as_value() } else { ruby.str_new(&unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy()).as_value() })
},
"TextSubtext" => { arity(args, 3)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::TextSubtext(text_0.pointer(), i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(if result.is_null() { ruby.qnil().as_value() } else { ruby.str_new(&unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy()).as_value() })
},
"TextRemoveSpaces" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::TextRemoveSpaces(text_0.pointer()) };
Ok(if result.is_null() { ruby.qnil().as_value() } else { ruby.str_new(&unsafe { std::ffi::CStr::from_ptr(result) }.to_string_lossy()).as_value() })
},
"GetTextBetween" => { arity(args, 3)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let text_2 = string_argument(args[2])?;
let result = unsafe { ffi::GetTextBetween(text_0.pointer(), text_1.pointer(), text_2.pointer()) };
Ok(((result) as usize).into_value_with(ruby))
},
"TextReplace" => { arity(args, 3)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let text_2 = string_argument(args[2])?;
let result = unsafe { ffi::TextReplace(text_0.pointer(), text_1.pointer(), text_2.pointer()) };
Ok(((result) as usize).into_value_with(ruby))
},
"TextReplaceAlloc" => { arity(args, 3)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let text_2 = string_argument(args[2])?;
let result = unsafe { ffi::TextReplaceAlloc(text_0.pointer(), text_1.pointer(), text_2.pointer()) };
Ok(((result) as usize).into_value_with(ruby))
},
"TextReplaceBetween" => { arity(args, 4)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let text_2 = string_argument(args[2])?;
let text_3 = string_argument(args[3])?;
let result = unsafe { ffi::TextReplaceBetween(text_0.pointer(), text_1.pointer(), text_2.pointer(), text_3.pointer()) };
Ok(((result) as usize).into_value_with(ruby))
},
"TextReplaceBetweenAlloc" => { arity(args, 4)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let text_2 = string_argument(args[2])?;
let text_3 = string_argument(args[3])?;
let result = unsafe { ffi::TextReplaceBetweenAlloc(text_0.pointer(), text_1.pointer(), text_2.pointer(), text_3.pointer()) };
Ok(((result) as usize).into_value_with(ruby))
},
"TextInsert" => { arity(args, 3)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::TextInsert(text_0.pointer(), text_1.pointer(), i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(((result) as usize).into_value_with(ruby))
},
"TextInsertAlloc" => { arity(args, 3)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::TextInsertAlloc(text_0.pointer(), text_1.pointer(), i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(((result) as usize).into_value_with(ruby))
},
"TextJoin" => { arity(args, 3)?;
let text_2 = string_argument(args[2])?;
let result = unsafe { ffi::TextJoin(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, text_2.pointer()) };
Ok(((result) as usize).into_value_with(ruby))
},
"TextSplit" => { arity(args, 3)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::TextSplit(text_0.pointer(), std::ffi::c_char::try_from(i64::try_convert(args[1])?).map_err(|_| argument("char out of range"))?, address(args[2])? as _) };
Ok(((result) as usize).into_value_with(ruby))
},
"TextAppend" => { arity(args, 3)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::TextAppend(address(args[0])? as _, text_1.pointer(), address(args[2])? as _) };
Ok(ruby.qnil().as_value())
},
"TextFindIndex" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::TextFindIndex(text_0.pointer(), text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"TextToUpper" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::TextToUpper(text_0.pointer()) };
Ok(((result) as usize).into_value_with(ruby))
},
"TextToLower" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::TextToLower(text_0.pointer()) };
Ok(((result) as usize).into_value_with(ruby))
},
"TextToPascal" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::TextToPascal(text_0.pointer()) };
Ok(((result) as usize).into_value_with(ruby))
},
"TextToSnake" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::TextToSnake(text_0.pointer()) };
Ok(((result) as usize).into_value_with(ruby))
},
"TextToCamel" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::TextToCamel(text_0.pointer()) };
Ok(((result) as usize).into_value_with(ruby))
},
"TextToInteger" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::TextToInteger(text_0.pointer()) };
Ok((result).into_value_with(ruby))
},
"TextToFloat" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::TextToFloat(text_0.pointer()) };
Ok((result).into_value_with(ruby))
},
"DrawLine3D" => { arity(args, 3)?;
let result = unsafe { ffi::DrawLine3D(read_Vector3(args[0])?, read_Vector3(args[1])?, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"DrawPoint3D" => { arity(args, 2)?;
let result = unsafe { ffi::DrawPoint3D(read_Vector3(args[0])?, read_Color(args[1])?) };
Ok(ruby.qnil().as_value())
},
"DrawCircle3D" => { arity(args, 5)?;
let result = unsafe { ffi::DrawCircle3D(read_Vector3(args[0])?, f64::try_convert(args[1])? as f32, read_Vector3(args[2])?, f64::try_convert(args[3])? as f32, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawTriangle3D" => { arity(args, 4)?;
let result = unsafe { ffi::DrawTriangle3D(read_Vector3(args[0])?, read_Vector3(args[1])?, read_Vector3(args[2])?, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawTriangleStrip3D" => { arity(args, 3)?;
let result = unsafe { ffi::DrawTriangleStrip3D(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"DrawCube" => { arity(args, 5)?;
let result = unsafe { ffi::DrawCube(read_Vector3(args[0])?, f64::try_convert(args[1])? as f32, f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawCubeV" => { arity(args, 3)?;
let result = unsafe { ffi::DrawCubeV(read_Vector3(args[0])?, read_Vector3(args[1])?, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"DrawCubeWires" => { arity(args, 5)?;
let result = unsafe { ffi::DrawCubeWires(read_Vector3(args[0])?, f64::try_convert(args[1])? as f32, f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawCubeWiresV" => { arity(args, 3)?;
let result = unsafe { ffi::DrawCubeWiresV(read_Vector3(args[0])?, read_Vector3(args[1])?, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"DrawSphere" => { arity(args, 3)?;
let result = unsafe { ffi::DrawSphere(read_Vector3(args[0])?, f64::try_convert(args[1])? as f32, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"DrawSphereEx" => { arity(args, 5)?;
let result = unsafe { ffi::DrawSphereEx(read_Vector3(args[0])?, f64::try_convert(args[1])? as f32, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawSphereWires" => { arity(args, 5)?;
let result = unsafe { ffi::DrawSphereWires(read_Vector3(args[0])?, f64::try_convert(args[1])? as f32, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawCylinder" => { arity(args, 6)?;
let result = unsafe { ffi::DrawCylinder(read_Vector3(args[0])?, f64::try_convert(args[1])? as f32, f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawCylinderEx" => { arity(args, 6)?;
let result = unsafe { ffi::DrawCylinderEx(read_Vector3(args[0])?, read_Vector3(args[1])?, f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawCylinderWires" => { arity(args, 6)?;
let result = unsafe { ffi::DrawCylinderWires(read_Vector3(args[0])?, f64::try_convert(args[1])? as f32, f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawCylinderWiresEx" => { arity(args, 6)?;
let result = unsafe { ffi::DrawCylinderWiresEx(read_Vector3(args[0])?, read_Vector3(args[1])?, f64::try_convert(args[2])? as f32, f64::try_convert(args[3])? as f32, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawCapsule" => { arity(args, 6)?;
let result = unsafe { ffi::DrawCapsule(read_Vector3(args[0])?, read_Vector3(args[1])?, f64::try_convert(args[2])? as f32, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawCapsuleWires" => { arity(args, 6)?;
let result = unsafe { ffi::DrawCapsuleWires(read_Vector3(args[0])?, read_Vector3(args[1])?, f64::try_convert(args[2])? as f32, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawPlane" => { arity(args, 3)?;
let result = unsafe { ffi::DrawPlane(read_Vector3(args[0])?, read_Vector2(args[1])?, read_Color(args[2])?) };
Ok(ruby.qnil().as_value())
},
"DrawRay" => { arity(args, 2)?;
let result = unsafe { ffi::DrawRay(read_Ray(args[0])?, read_Color(args[1])?) };
Ok(ruby.qnil().as_value())
},
"DrawGrid" => { arity(args, 2)?;
let result = unsafe { ffi::DrawGrid(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[1])? as f32) };
Ok(ruby.qnil().as_value())
},
"LoadModel" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadModel(text_0.pointer()) };
Ok(write_Model(ruby, result)?)
},
"LoadModelFromMesh" => { arity(args, 1)?;
let result = unsafe { ffi::LoadModelFromMesh(read_Mesh(args[0])?) };
Ok(write_Model(ruby, result)?)
},
"IsModelValid" => { arity(args, 1)?;
let result = unsafe { ffi::IsModelValid(read_Model(args[0])?) };
Ok((result).into_value_with(ruby))
},
"UnloadModel" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadModel(read_Model(args[0])?) };
Ok(ruby.qnil().as_value())
},
"GetModelBoundingBox" => { arity(args, 1)?;
let result = unsafe { ffi::GetModelBoundingBox(read_Model(args[0])?) };
Ok(write_BoundingBox(ruby, result)?)
},
"DrawModel" => { arity(args, 4)?;
let result = unsafe { ffi::DrawModel(read_Model(args[0])?, read_Vector3(args[1])?, f64::try_convert(args[2])? as f32, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawModelEx" => { arity(args, 6)?;
let result = unsafe { ffi::DrawModelEx(read_Model(args[0])?, read_Vector3(args[1])?, read_Vector3(args[2])?, f64::try_convert(args[3])? as f32, read_Vector3(args[4])?, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawModelWires" => { arity(args, 4)?;
let result = unsafe { ffi::DrawModelWires(read_Model(args[0])?, read_Vector3(args[1])?, f64::try_convert(args[2])? as f32, read_Color(args[3])?) };
Ok(ruby.qnil().as_value())
},
"DrawModelWiresEx" => { arity(args, 6)?;
let result = unsafe { ffi::DrawModelWiresEx(read_Model(args[0])?, read_Vector3(args[1])?, read_Vector3(args[2])?, f64::try_convert(args[3])? as f32, read_Vector3(args[4])?, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawBoundingBox" => { arity(args, 2)?;
let result = unsafe { ffi::DrawBoundingBox(read_BoundingBox(args[0])?, read_Color(args[1])?) };
Ok(ruby.qnil().as_value())
},
"DrawBillboard" => { arity(args, 5)?;
let result = unsafe { ffi::DrawBillboard(read_Camera3D(args[0])?, read_Texture(args[1])?, read_Vector3(args[2])?, f64::try_convert(args[3])? as f32, read_Color(args[4])?) };
Ok(ruby.qnil().as_value())
},
"DrawBillboardRec" => { arity(args, 6)?;
let result = unsafe { ffi::DrawBillboardRec(read_Camera3D(args[0])?, read_Texture(args[1])?, read_Rectangle(args[2])?, read_Vector3(args[3])?, read_Vector2(args[4])?, read_Color(args[5])?) };
Ok(ruby.qnil().as_value())
},
"DrawBillboardPro" => { arity(args, 9)?;
let result = unsafe { ffi::DrawBillboardPro(read_Camera3D(args[0])?, read_Texture(args[1])?, read_Rectangle(args[2])?, read_Vector3(args[3])?, read_Vector3(args[4])?, read_Vector2(args[5])?, read_Vector2(args[6])?, f64::try_convert(args[7])? as f32, read_Color(args[8])?) };
Ok(ruby.qnil().as_value())
},
"UploadMesh" => { arity(args, 2)?;
let result = unsafe { ffi::UploadMesh(address(args[0])? as _, bool::try_convert(args[1])?) };
Ok(ruby.qnil().as_value())
},
"UpdateMeshBuffer" => { arity(args, 5)?;
let result = unsafe { ffi::UpdateMeshBuffer(read_Mesh(args[0])?, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, address(args[2])? as _, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[4])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"UnloadMesh" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadMesh(read_Mesh(args[0])?) };
Ok(ruby.qnil().as_value())
},
"DrawMesh" => { arity(args, 3)?;
let result = unsafe { ffi::DrawMesh(read_Mesh(args[0])?, read_Material(args[1])?, read_Matrix(args[2])?) };
Ok(ruby.qnil().as_value())
},
"DrawMeshInstanced" => { arity(args, 4)?;
let result = unsafe { ffi::DrawMeshInstanced(read_Mesh(args[0])?, read_Material(args[1])?, address(args[2])? as _, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"GetMeshBoundingBox" => { arity(args, 1)?;
let result = unsafe { ffi::GetMeshBoundingBox(read_Mesh(args[0])?) };
Ok(write_BoundingBox(ruby, result)?)
},
"GenMeshTangents" => { arity(args, 1)?;
let result = unsafe { ffi::GenMeshTangents(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"ExportMesh" => { arity(args, 2)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::ExportMesh(read_Mesh(args[0])?, text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"ExportMeshAsCode" => { arity(args, 2)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::ExportMeshAsCode(read_Mesh(args[0])?, text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"GenMeshPoly" => { arity(args, 2)?;
let result = unsafe { ffi::GenMeshPoly(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?, f64::try_convert(args[1])? as f32) };
Ok(write_Mesh(ruby, result)?)
},
"GenMeshPlane" => { arity(args, 4)?;
let result = unsafe { ffi::GenMeshPlane(f64::try_convert(args[0])? as f32, f64::try_convert(args[1])? as f32, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Mesh(ruby, result)?)
},
"GenMeshCube" => { arity(args, 3)?;
let result = unsafe { ffi::GenMeshCube(f64::try_convert(args[0])? as f32, f64::try_convert(args[1])? as f32, f64::try_convert(args[2])? as f32) };
Ok(write_Mesh(ruby, result)?)
},
"GenMeshSphere" => { arity(args, 3)?;
let result = unsafe { ffi::GenMeshSphere(f64::try_convert(args[0])? as f32, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Mesh(ruby, result)?)
},
"GenMeshHemiSphere" => { arity(args, 3)?;
let result = unsafe { ffi::GenMeshHemiSphere(f64::try_convert(args[0])? as f32, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Mesh(ruby, result)?)
},
"GenMeshCylinder" => { arity(args, 3)?;
let result = unsafe { ffi::GenMeshCylinder(f64::try_convert(args[0])? as f32, f64::try_convert(args[1])? as f32, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Mesh(ruby, result)?)
},
"GenMeshCone" => { arity(args, 3)?;
let result = unsafe { ffi::GenMeshCone(f64::try_convert(args[0])? as f32, f64::try_convert(args[1])? as f32, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Mesh(ruby, result)?)
},
"GenMeshTorus" => { arity(args, 4)?;
let result = unsafe { ffi::GenMeshTorus(f64::try_convert(args[0])? as f32, f64::try_convert(args[1])? as f32, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Mesh(ruby, result)?)
},
"GenMeshKnot" => { arity(args, 4)?;
let result = unsafe { ffi::GenMeshKnot(f64::try_convert(args[0])? as f32, f64::try_convert(args[1])? as f32, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Mesh(ruby, result)?)
},
"GenMeshHeightmap" => { arity(args, 2)?;
let result = unsafe { ffi::GenMeshHeightmap(read_Image(args[0])?, read_Vector3(args[1])?) };
Ok(write_Mesh(ruby, result)?)
},
"GenMeshCubicmap" => { arity(args, 2)?;
let result = unsafe { ffi::GenMeshCubicmap(read_Image(args[0])?, read_Vector3(args[1])?) };
Ok(write_Mesh(ruby, result)?)
},
"LoadMaterials" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadMaterials(text_0.pointer(), address(args[1])? as _) };
Ok(((result) as usize).into_value_with(ruby))
},
"LoadMaterialDefault" => { arity(args, 0)?;
let result = unsafe { ffi::LoadMaterialDefault() };
Ok(write_Material(ruby, result)?)
},
"IsMaterialValid" => { arity(args, 1)?;
let result = unsafe { ffi::IsMaterialValid(read_Material(args[0])?) };
Ok((result).into_value_with(ruby))
},
"UnloadMaterial" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadMaterial(read_Material(args[0])?) };
Ok(ruby.qnil().as_value())
},
"SetMaterialTexture" => { arity(args, 3)?;
let result = unsafe { ffi::SetMaterialTexture(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, read_Texture(args[2])?) };
Ok(ruby.qnil().as_value())
},
"SetModelMeshMaterial" => { arity(args, 3)?;
let result = unsafe { ffi::SetModelMeshMaterial(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"LoadModelAnimations" => { arity(args, 2)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadModelAnimations(text_0.pointer(), address(args[1])? as _) };
Ok(((result) as usize).into_value_with(ruby))
},
"UpdateModelAnimation" => { arity(args, 3)?;
let result = unsafe { ffi::UpdateModelAnimation(read_Model(args[0])?, read_ModelAnimation(args[1])?, f64::try_convert(args[2])? as f32) };
Ok(ruby.qnil().as_value())
},
"UpdateModelAnimationEx" => { arity(args, 6)?;
let result = unsafe { ffi::UpdateModelAnimationEx(read_Model(args[0])?, read_ModelAnimation(args[1])?, f64::try_convert(args[2])? as f32, read_ModelAnimation(args[3])?, f64::try_convert(args[4])? as f32, f64::try_convert(args[5])? as f32) };
Ok(ruby.qnil().as_value())
},
"UnloadModelAnimations" => { arity(args, 2)?;
let result = unsafe { ffi::UnloadModelAnimations(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"IsModelAnimationValid" => { arity(args, 2)?;
let result = unsafe { ffi::IsModelAnimationValid(read_Model(args[0])?, read_ModelAnimation(args[1])?) };
Ok((result).into_value_with(ruby))
},
"CheckCollisionSpheres" => { arity(args, 4)?;
let result = unsafe { ffi::CheckCollisionSpheres(read_Vector3(args[0])?, f64::try_convert(args[1])? as f32, read_Vector3(args[2])?, f64::try_convert(args[3])? as f32) };
Ok((result).into_value_with(ruby))
},
"CheckCollisionBoxes" => { arity(args, 2)?;
let result = unsafe { ffi::CheckCollisionBoxes(read_BoundingBox(args[0])?, read_BoundingBox(args[1])?) };
Ok((result).into_value_with(ruby))
},
"CheckCollisionBoxSphere" => { arity(args, 3)?;
let result = unsafe { ffi::CheckCollisionBoxSphere(read_BoundingBox(args[0])?, read_Vector3(args[1])?, f64::try_convert(args[2])? as f32) };
Ok((result).into_value_with(ruby))
},
"GetRayCollisionSphere" => { arity(args, 3)?;
let result = unsafe { ffi::GetRayCollisionSphere(read_Ray(args[0])?, read_Vector3(args[1])?, f64::try_convert(args[2])? as f32) };
Ok(write_RayCollision(ruby, result)?)
},
"GetRayCollisionBox" => { arity(args, 2)?;
let result = unsafe { ffi::GetRayCollisionBox(read_Ray(args[0])?, read_BoundingBox(args[1])?) };
Ok(write_RayCollision(ruby, result)?)
},
"GetRayCollisionMesh" => { arity(args, 3)?;
let result = unsafe { ffi::GetRayCollisionMesh(read_Ray(args[0])?, read_Mesh(args[1])?, read_Matrix(args[2])?) };
Ok(write_RayCollision(ruby, result)?)
},
"GetRayCollisionTriangle" => { arity(args, 4)?;
let result = unsafe { ffi::GetRayCollisionTriangle(read_Ray(args[0])?, read_Vector3(args[1])?, read_Vector3(args[2])?, read_Vector3(args[3])?) };
Ok(write_RayCollision(ruby, result)?)
},
"GetRayCollisionQuad" => { arity(args, 5)?;
let result = unsafe { ffi::GetRayCollisionQuad(read_Ray(args[0])?, read_Vector3(args[1])?, read_Vector3(args[2])?, read_Vector3(args[3])?, read_Vector3(args[4])?) };
Ok(write_RayCollision(ruby, result)?)
},
"InitAudioDevice" => { arity(args, 0)?;
let result = crate::game::init_audio_device()?;
Ok(ruby.qnil().as_value())
},
"CloseAudioDevice" => { arity(args, 0)?;
let result = crate::game::close_audio_device()?;
Ok(ruby.qnil().as_value())
},
"IsAudioDeviceReady" => { arity(args, 0)?;
let result = unsafe { ffi::IsAudioDeviceReady() };
Ok((result).into_value_with(ruby))
},
"SetMasterVolume" => { arity(args, 1)?;
let result = unsafe { ffi::SetMasterVolume(f64::try_convert(args[0])? as f32) };
Ok(ruby.qnil().as_value())
},
"GetMasterVolume" => { arity(args, 0)?;
let result = unsafe { ffi::GetMasterVolume() };
Ok((result).into_value_with(ruby))
},
"LoadWave" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadWave(text_0.pointer()) };
Ok(write_Wave(ruby, result)?)
},
"LoadWaveFromMemory" => { arity(args, 3)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadWaveFromMemory(text_0.pointer(), address(args[1])? as _, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Wave(ruby, result)?)
},
"IsWaveValid" => { arity(args, 1)?;
let result = unsafe { ffi::IsWaveValid(read_Wave(args[0])?) };
Ok((result).into_value_with(ruby))
},
"LoadSound" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadSound(text_0.pointer()) };
Ok(write_Sound(ruby, result)?)
},
"LoadSoundFromWave" => { arity(args, 1)?;
let result = unsafe { ffi::LoadSoundFromWave(read_Wave(args[0])?) };
Ok(write_Sound(ruby, result)?)
},
"LoadSoundAlias" => { arity(args, 1)?;
let result = unsafe { ffi::LoadSoundAlias(read_Sound(args[0])?) };
Ok(write_Sound(ruby, result)?)
},
"IsSoundValid" => { arity(args, 1)?;
let result = unsafe { ffi::IsSoundValid(read_Sound(args[0])?) };
Ok((result).into_value_with(ruby))
},
"UpdateSound" => { arity(args, 3)?;
let result = unsafe { ffi::UpdateSound(read_Sound(args[0])?, address(args[1])? as _, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"UnloadWave" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadWave(read_Wave(args[0])?) };
Ok(ruby.qnil().as_value())
},
"UnloadSound" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadSound(read_Sound(args[0])?) };
Ok(ruby.qnil().as_value())
},
"UnloadSoundAlias" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadSoundAlias(read_Sound(args[0])?) };
Ok(ruby.qnil().as_value())
},
"ExportWave" => { arity(args, 2)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::ExportWave(read_Wave(args[0])?, text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"ExportWaveAsCode" => { arity(args, 2)?;
let text_1 = string_argument(args[1])?;
let result = unsafe { ffi::ExportWaveAsCode(read_Wave(args[0])?, text_1.pointer()) };
Ok((result).into_value_with(ruby))
},
"PlaySound" => { arity(args, 1)?;
let result = unsafe { ffi::PlaySound(read_Sound(args[0])?) };
Ok(ruby.qnil().as_value())
},
"StopSound" => { arity(args, 1)?;
let result = unsafe { ffi::StopSound(read_Sound(args[0])?) };
Ok(ruby.qnil().as_value())
},
"PauseSound" => { arity(args, 1)?;
let result = unsafe { ffi::PauseSound(read_Sound(args[0])?) };
Ok(ruby.qnil().as_value())
},
"ResumeSound" => { arity(args, 1)?;
let result = unsafe { ffi::ResumeSound(read_Sound(args[0])?) };
Ok(ruby.qnil().as_value())
},
"IsSoundPlaying" => { arity(args, 1)?;
let result = unsafe { ffi::IsSoundPlaying(read_Sound(args[0])?) };
Ok((result).into_value_with(ruby))
},
"SetSoundVolume" => { arity(args, 2)?;
let result = unsafe { ffi::SetSoundVolume(read_Sound(args[0])?, f64::try_convert(args[1])? as f32) };
Ok(ruby.qnil().as_value())
},
"SetSoundPitch" => { arity(args, 2)?;
let result = unsafe { ffi::SetSoundPitch(read_Sound(args[0])?, f64::try_convert(args[1])? as f32) };
Ok(ruby.qnil().as_value())
},
"SetSoundPan" => { arity(args, 2)?;
let result = unsafe { ffi::SetSoundPan(read_Sound(args[0])?, f64::try_convert(args[1])? as f32) };
Ok(ruby.qnil().as_value())
},
"WaveCopy" => { arity(args, 1)?;
let result = unsafe { ffi::WaveCopy(read_Wave(args[0])?) };
Ok(write_Wave(ruby, result)?)
},
"WaveCrop" => { arity(args, 3)?;
let result = unsafe { ffi::WaveCrop(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"WaveFormat" => { arity(args, 4)?;
let result = unsafe { ffi::WaveFormat(address(args[0])? as _, i32::try_from(i64::try_convert(args[1])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?, i32::try_from(i64::try_convert(args[3])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"LoadWaveSamples" => { arity(args, 1)?;
let result = unsafe { ffi::LoadWaveSamples(read_Wave(args[0])?) };
Ok(((result) as usize).into_value_with(ruby))
},
"UnloadWaveSamples" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadWaveSamples(address(args[0])? as _) };
Ok(ruby.qnil().as_value())
},
"LoadMusicStream" => { arity(args, 1)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadMusicStream(text_0.pointer()) };
Ok(write_Music(ruby, result)?)
},
"LoadMusicStreamFromMemory" => { arity(args, 3)?;
let text_0 = string_argument(args[0])?;
let result = unsafe { ffi::LoadMusicStreamFromMemory(text_0.pointer(), address(args[1])? as _, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(write_Music(ruby, result)?)
},
"IsMusicValid" => { arity(args, 1)?;
let result = unsafe { ffi::IsMusicValid(read_Music(args[0])?) };
Ok((result).into_value_with(ruby))
},
"UnloadMusicStream" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadMusicStream(read_Music(args[0])?) };
Ok(ruby.qnil().as_value())
},
"PlayMusicStream" => { arity(args, 1)?;
let result = unsafe { ffi::PlayMusicStream(read_Music(args[0])?) };
Ok(ruby.qnil().as_value())
},
"IsMusicStreamPlaying" => { arity(args, 1)?;
let result = unsafe { ffi::IsMusicStreamPlaying(read_Music(args[0])?) };
Ok((result).into_value_with(ruby))
},
"UpdateMusicStream" => { arity(args, 1)?;
let result = unsafe { ffi::UpdateMusicStream(read_Music(args[0])?) };
Ok(ruby.qnil().as_value())
},
"StopMusicStream" => { arity(args, 1)?;
let result = unsafe { ffi::StopMusicStream(read_Music(args[0])?) };
Ok(ruby.qnil().as_value())
},
"PauseMusicStream" => { arity(args, 1)?;
let result = unsafe { ffi::PauseMusicStream(read_Music(args[0])?) };
Ok(ruby.qnil().as_value())
},
"ResumeMusicStream" => { arity(args, 1)?;
let result = unsafe { ffi::ResumeMusicStream(read_Music(args[0])?) };
Ok(ruby.qnil().as_value())
},
"SeekMusicStream" => { arity(args, 2)?;
let result = unsafe { ffi::SeekMusicStream(read_Music(args[0])?, f64::try_convert(args[1])? as f32) };
Ok(ruby.qnil().as_value())
},
"SetMusicVolume" => { arity(args, 2)?;
let result = unsafe { ffi::SetMusicVolume(read_Music(args[0])?, f64::try_convert(args[1])? as f32) };
Ok(ruby.qnil().as_value())
},
"SetMusicPitch" => { arity(args, 2)?;
let result = unsafe { ffi::SetMusicPitch(read_Music(args[0])?, f64::try_convert(args[1])? as f32) };
Ok(ruby.qnil().as_value())
},
"SetMusicPan" => { arity(args, 2)?;
let result = unsafe { ffi::SetMusicPan(read_Music(args[0])?, f64::try_convert(args[1])? as f32) };
Ok(ruby.qnil().as_value())
},
"GetMusicTimeLength" => { arity(args, 1)?;
let result = unsafe { ffi::GetMusicTimeLength(read_Music(args[0])?) };
Ok((result).into_value_with(ruby))
},
"GetMusicTimePlayed" => { arity(args, 1)?;
let result = unsafe { ffi::GetMusicTimePlayed(read_Music(args[0])?) };
Ok((result).into_value_with(ruby))
},
"LoadAudioStream" => { arity(args, 3)?;
let result = unsafe { ffi::LoadAudioStream(u32::try_from(u64::try_convert(args[0])?).map_err(|_| argument("unsigned int out of range"))?, u32::try_from(u64::try_convert(args[1])?).map_err(|_| argument("unsigned int out of range"))?, u32::try_from(u64::try_convert(args[2])?).map_err(|_| argument("unsigned int out of range"))?) };
Ok(write_AudioStream(ruby, result)?)
},
"IsAudioStreamValid" => { arity(args, 1)?;
let result = unsafe { ffi::IsAudioStreamValid(read_AudioStream(args[0])?) };
Ok((result).into_value_with(ruby))
},
"UnloadAudioStream" => { arity(args, 1)?;
let result = unsafe { ffi::UnloadAudioStream(read_AudioStream(args[0])?) };
Ok(ruby.qnil().as_value())
},
"UpdateAudioStream" => { arity(args, 3)?;
let result = unsafe { ffi::UpdateAudioStream(read_AudioStream(args[0])?, address(args[1])? as _, i32::try_from(i64::try_convert(args[2])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"IsAudioStreamProcessed" => { arity(args, 1)?;
let result = unsafe { ffi::IsAudioStreamProcessed(read_AudioStream(args[0])?) };
Ok((result).into_value_with(ruby))
},
"PlayAudioStream" => { arity(args, 1)?;
let result = unsafe { ffi::PlayAudioStream(read_AudioStream(args[0])?) };
Ok(ruby.qnil().as_value())
},
"PauseAudioStream" => { arity(args, 1)?;
let result = unsafe { ffi::PauseAudioStream(read_AudioStream(args[0])?) };
Ok(ruby.qnil().as_value())
},
"ResumeAudioStream" => { arity(args, 1)?;
let result = unsafe { ffi::ResumeAudioStream(read_AudioStream(args[0])?) };
Ok(ruby.qnil().as_value())
},
"IsAudioStreamPlaying" => { arity(args, 1)?;
let result = unsafe { ffi::IsAudioStreamPlaying(read_AudioStream(args[0])?) };
Ok((result).into_value_with(ruby))
},
"StopAudioStream" => { arity(args, 1)?;
let result = unsafe { ffi::StopAudioStream(read_AudioStream(args[0])?) };
Ok(ruby.qnil().as_value())
},
"SetAudioStreamVolume" => { arity(args, 2)?;
let result = unsafe { ffi::SetAudioStreamVolume(read_AudioStream(args[0])?, f64::try_convert(args[1])? as f32) };
Ok(ruby.qnil().as_value())
},
"SetAudioStreamPitch" => { arity(args, 2)?;
let result = unsafe { ffi::SetAudioStreamPitch(read_AudioStream(args[0])?, f64::try_convert(args[1])? as f32) };
Ok(ruby.qnil().as_value())
},
"SetAudioStreamPan" => { arity(args, 2)?;
let result = unsafe { ffi::SetAudioStreamPan(read_AudioStream(args[0])?, f64::try_convert(args[1])? as f32) };
Ok(ruby.qnil().as_value())
},
"SetAudioStreamBufferSizeDefault" => { arity(args, 1)?;
let result = unsafe { ffi::SetAudioStreamBufferSizeDefault(i32::try_from(i64::try_convert(args[0])?).map_err(|_| argument("int out of range"))?) };
Ok(ruby.qnil().as_value())
},
"SetAudioStreamCallback" => { arity(args, 2)?;
let result = unsafe { ffi::SetAudioStreamCallback(read_AudioStream(args[0])?, unsafe { std::mem::transmute::<usize, ffi::AudioCallback>(address(args[1])?) }) };
Ok(ruby.qnil().as_value())
},
"AttachAudioStreamProcessor" => { arity(args, 2)?;
let result = unsafe { ffi::AttachAudioStreamProcessor(read_AudioStream(args[0])?, unsafe { std::mem::transmute::<usize, ffi::AudioCallback>(address(args[1])?) }) };
Ok(ruby.qnil().as_value())
},
"DetachAudioStreamProcessor" => { arity(args, 2)?;
let result = unsafe { ffi::DetachAudioStreamProcessor(read_AudioStream(args[0])?, unsafe { std::mem::transmute::<usize, ffi::AudioCallback>(address(args[1])?) }) };
Ok(ruby.qnil().as_value())
},
"AttachAudioMixedProcessor" => { arity(args, 1)?;
let result = unsafe { ffi::AttachAudioMixedProcessor(unsafe { std::mem::transmute::<usize, ffi::AudioCallback>(address(args[0])?) }) };
Ok(ruby.qnil().as_value())
},
"DetachAudioMixedProcessor" => { arity(args, 1)?;
let result = unsafe { ffi::DetachAudioMixedProcessor(unsafe { std::mem::transmute::<usize, ffi::AudioCallback>(address(args[0])?) }) };
Ok(ruby.qnil().as_value())
},
_ => Err(argument(format!("Unknown raylib function: {name}"))), } }

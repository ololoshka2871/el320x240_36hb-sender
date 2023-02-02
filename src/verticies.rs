// rust code representation of the VertexInput struct in the shader
#[repr(C)]
// Не дать rust переставить поля местами, будет как в C и как в шейдере
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct MyVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

// Дескриптор того как читать драйверу массив MyVertex'ов
impl MyVertex {
    // 0 and 1 are the locations of the attributes in the shader (@location(n))
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];
    pub(crate) fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MyVertex>() as wgpu::BufferAddress, // Размер 1 элемента в байтах
            step_mode: wgpu::VertexStepMode::Vertex, // 1 элемент на 1 вершину
            attributes: &Self::ATTRIBUTES,
        }
    }
}

// Набор вершин для постороения прямоугольника, на который будем рисовать
pub(crate) const VERTICES: &[MyVertex] = &[
    MyVertex {
        position: [-1.0, 1.0],
        tex_coords: [0.0, 0.0],
    },
    MyVertex {
        position: [-1.0, -1.0],
        tex_coords: [0.0, 1.0],
    },
    MyVertex {
        position: [1.0, 1.0],
        tex_coords: [1.0, 0.0],
    },
    MyVertex {
        position: [1.0, -1.0],
        tex_coords: [1.0, 1.0],
    },
];

// Индексы для построения прямоугольника
// 0 1 2 (первый треугольник)
// 1 2 3 (второй треугольник)
pub(crate) const INDICES: &[u16] = &[0, 1, 2, 3];
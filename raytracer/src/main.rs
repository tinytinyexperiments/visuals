use std::fs::File;
use std::io::{BufWriter, Write};

use rand::Rng;

#[derive(Clone, Copy, Debug)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

impl Vec3 {
    fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    fn length(&self) -> f64 {
        self.length_squared().sqrt()
    }

    fn length_squared(&self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    fn dot(&self, other: &Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn unit(self) -> Self {
        let len = self.length();
        Self::new(self.x / len, self.y / len, self.z / len)
    }
}

use std::ops::{Add, Mul, Sub};

impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, t: f64) -> Self::Output {
        Self::new(self.x * t, self.y * t, self.z * t)
    }
}

type Color = Vec3;
type Point3 = Vec3;

#[derive(Clone, Copy)]
struct Ray {
    origin: Point3,
    direction: Vec3,
}

impl Ray {
    fn new(origin: Point3, direction: Vec3) -> Self {
        Self { origin, direction }
    }

    fn at(&self, t: f64) -> Point3 {
        self.origin + self.direction * t
    }
}

#[derive(Clone, Copy)]
struct Sphere {
    center: Point3,
    radius: f64,
}

fn hit_sphere(center: Point3, radius: f64, r: &Ray) -> Option<f64> {
    let oc = r.origin - center;
    let a = r.direction.length_squared();
    let half_b = oc.dot(&r.direction);
    let c = oc.length_squared() - radius * radius;
    let discriminant = half_b * half_b - a * c;
    if discriminant < 0.0 {
        None
    } else {
        Some((-half_b - discriminant.sqrt()) / a)
    }
}

fn ray_color(r: &Ray, world: &[Sphere]) -> Color {
    let mut closest_so_far = f64::INFINITY;
    let mut hit_sphere_idx: Option<usize> = None;

    for (i, s) in world.iter().enumerate() {
        if let Some(t) = hit_sphere(s.center, s.radius, r) {
            if t > 0.001 && t < closest_so_far {
                closest_so_far = t;
                hit_sphere_idx = Some(i);
            }
        }
    }

    if let Some(i) = hit_sphere_idx {
        let s = world[i];
        let p = r.at(closest_so_far);
        let n = (p - s.center).unit();
        return Color::new(n.x + 1.0, n.y + 1.0, n.z + 1.0) * 0.5;
    }

    // background gradient
    let unit_dir = r.direction.unit();
    let t = 0.5 * (unit_dir.y + 1.0);
    Color::new(1.0, 1.0, 1.0) * (1.0 - t) + Color::new(0.5, 0.7, 1.0) * t
}

fn write_color<W: Write>(out: &mut W, pixel_color: Color) -> std::io::Result<()> {
    // simple gamma correction (gamma 2.0)
    let r = pixel_color.x.clamp(0.0, 0.999).sqrt();
    let g = pixel_color.y.clamp(0.0, 0.999).sqrt();
    let b = pixel_color.z.clamp(0.0, 0.999).sqrt();

    let ir = (255.999 * r) as i32;
    let ig = (255.999 * g) as i32;
    let ib = (255.999 * b) as i32;
    writeln!(out, "{ir} {ig} {ib}")
}

fn main() -> std::io::Result<()> {
    // Image
    let aspect_ratio = 16.0 / 9.0;
    let image_width: i32 = 400;
    let image_height: i32 = ((image_width as f64) / aspect_ratio) as i32;
    let samples_per_pixel = 20;

    // Camera
    let viewport_height = 2.0;
    let viewport_width = aspect_ratio * viewport_height;
    let focal_length = 1.0;

    let origin = Point3::new(0.0, 0.0, 0.0);
    let horizontal = Vec3::new(viewport_width, 0.0, 0.0);
    let vertical = Vec3::new(0.0, viewport_height, 0.0);
    let lower_left_corner = origin
        - horizontal * 0.5
        - vertical * 0.5
        - Vec3::new(0.0, 0.0, focal_length);

    // World: ground + three spheres
    let world = vec![
        Sphere {
            center: Point3::new(0.0, 0.0, -1.0),
            radius: 0.5,
        },
        Sphere {
            center: Point3::new(0.0, -100.5, -1.0),
            radius: 100.0,
        },
        Sphere {
            center: Point3::new(1.0, 0.0, -1.5),
            radius: 0.5,
        },
        Sphere {
            center: Point3::new(-1.0, 0.0, -1.5),
            radius: 0.5,
        },
    ];

    let file = File::create("image.ppm")?;
    let mut writer = BufWriter::new(file);

    writeln!(writer, "P3")?;
    writeln!(writer, "{image_width} {image_height}")?;
    writeln!(writer, "255")?;

    let mut rng = rand::thread_rng();

    for j in (0..image_height).rev() {
        for i in 0..image_width {
            let mut pixel_color = Color::new(0.0, 0.0, 0.0);

            for _ in 0..samples_per_pixel {
                let u = (i as f64 + rng.gen::<f64>()) / (image_width - 1) as f64;
                let v = (j as f64 + rng.gen::<f64>()) / (image_height - 1) as f64;

                let r = Ray::new(
                    origin,
                    lower_left_corner + horizontal * u + vertical * v - origin,
                );

                pixel_color = pixel_color + ray_color(&r, &world);
            }

            let scale = 1.0 / samples_per_pixel as f64;
            pixel_color = pixel_color * scale;

            write_color(&mut writer, pixel_color)?;
        }
    }

    println!("Wrote image.ppm");
    Ok(())
}



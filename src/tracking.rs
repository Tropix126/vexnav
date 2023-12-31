use core::f64::consts::{FRAC_2_PI, PI};
use core::prelude::rust_2021::*;
use num_traits::real::Real;

use crate::{
    math::Vec2,
    devices::{Gyro, RotarySensor},
};

/// A system that tracks the absolute position and heading of a mobile robot.
pub trait Tracking: Send + Sync + 'static {
    fn forward_travel(&self) -> f64;

    fn heading(&self) -> f64;
    fn set_heading(&mut self, heading: f64);

    fn position(&self) -> Vec2;
    fn set_position(&mut self, position: Vec2);

    fn update(&mut self);
}

/// A struct representing a wheel attached to a rotary sensor.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackingWheel<T: RotarySensor> {
    pub sensor: T,
    pub wheel_diameter: f64,
    pub offset: f64,
    pub gearing: Option<f64>,
}

impl<T: RotarySensor> TrackingWheel<T> {
    pub fn new(sensor: T, wheel_diameter: f64, offset: f64, gearing: Option<f64>) -> Self {
        Self {
            sensor,
            wheel_diameter,
            offset,
            gearing,
        }
    }
}

impl<T: RotarySensor> TrackingWheel<T> {
    fn travel(&self) -> f64 {
        let wheel_circumference = self.wheel_diameter * PI;

        return (self.sensor.rotation().expect("Could not measure TrackingWheel sensor rotation.") / FRAC_2_PI)
            * self.gearing.unwrap_or(1.0)
            * wheel_circumference;
    }
}

#[derive(Debug, PartialEq)]
pub struct ParallelWheelTracking<T: RotarySensor, U: RotarySensor, V: Gyro> {
    position: Vec2,
    left_wheel: TrackingWheel<T>,
    right_wheel: TrackingWheel<U>,
    gyro: Option<V>,
    heading_offset: f64,
    prev_forward_travel: f64,
    prev_heading: f64,
}

impl<T: RotarySensor, U: RotarySensor, V: Gyro> ParallelWheelTracking<T, U, V> {
    pub fn new(
        origin: Vec2,
        heading: f64,
        left_wheel: TrackingWheel<T>,
        right_wheel: TrackingWheel<U>,
        gyro: Option<V>
    ) -> Self {
        Self {
            position: origin,
            left_wheel,
            right_wheel,
            gyro,
            heading_offset: heading,
            prev_forward_travel: 0.0,
            prev_heading: 0.0,
        }
    }
}

impl<T: RotarySensor, U: RotarySensor, V: Gyro> ParallelWheelTracking<T, U, V> {
    fn track_width(&self) -> f64 {
        self.left_wheel.offset + self.right_wheel.offset
    }
}

impl<T: RotarySensor, U: RotarySensor, V: Gyro> Tracking
    for ParallelWheelTracking<T, U, V>
{
    fn position(&self) -> Vec2 {
        self.position
    }

    fn set_position(&mut self, position: Vec2) {
        self.position = position;
    }

    fn forward_travel(&self) -> f64 {
        (self.left_wheel.travel() + self.right_wheel.travel()) / 2.0
    }

    fn heading(&self) -> f64 {
        self.heading_offset + if let Some(gyro) = &self.gyro {
            gyro.heading().unwrap_or_else(|_| {
                (self.right_wheel.travel() - self.left_wheel.travel()) / self.track_width()
            })
        } else {
            (self.right_wheel.travel() - self.left_wheel.travel()) / self.track_width()
        } % FRAC_2_PI
    }

    fn set_heading(&mut self, heading: f64) {
        self.heading_offset = heading - self.heading();
    }

    fn update(&mut self) {
        let forward_travel = self.forward_travel();
        let heading = self.heading();
        
        let delta_forward_travel = forward_travel - self.prev_forward_travel;
        let delta_heading = heading - self.prev_heading;
    
        // Find a position delta.
        // This is a vector relative to the previous position, and can be found by creating a vector with our
        // average forward travel as the y-axis, then rotating the y-axis about our current heading. This gives
        // a rough estimate of the change in position, but does not account for sideways motion.
        self.position += Vec2::from_polar(
            2.0 * (delta_forward_travel / delta_heading) * (heading / 2.0).sin(),
            (self.prev_heading + delta_heading) / 2.0
        );
        self.prev_forward_travel = forward_travel;
    }
}
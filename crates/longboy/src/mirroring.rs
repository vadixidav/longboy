use enum_map::Enum;

#[derive(Clone, Copy, Debug, Enum)]
pub enum Mirroring
{
    AudioVideo,
    Background,
    Voice,
}

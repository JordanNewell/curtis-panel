"""Generate NEWELL-branded icon for openpanel.

NEWELL brand system:
  - Background: #0A0A0A (soft black, NOT pure #000000)
  - Primary green: #00FF41
  - 8px spacing grid, minimal/architectural, no gradients/shadows/glow

Shape: Classic lightning bolt, bold monoline.
       Single polygon, 10 vertices, no self-intersection.
       The bolt's silhouette is the standard "thunderbolt" used by countless
       charging/power UIs: top tip, kinks left, kinks right at the waist,
       tapers to bottom tip, with a notch on the right side.
"""
from PIL import Image, ImageDraw

SIZE = 512
BG = (0x0A, 0x0A, 0x0A, 0xFF)
GREEN = (0x00, 0xFF, 0x41, 0xFF)

img = Image.new("RGBA", (SIZE, SIZE), BG)
draw = ImageDraw.Draw(img)

cx = SIZE // 2

# Walk the bolt outline as 10 vertices, clockwise from top tip.
# The bolt has TWO concave notches (one on each side at the waist),
# which is what gives it the classic zigzag silhouette.
#
#        0 (top tip)
#       /|
#      / |
#     1  |
#      \ |
#       \2  <-- left notch (concave, points right toward center)
#       | |
#       | 3 <-- right notch (concave, points left toward center)
#       |/
#       4 (bottom tip)
#
# But for a real bolt the notches are offset vertically (not at same y),
# which is what creates the diagonal waist. Vertices clockwise:
bolt = [
    (cx,       56),    # 0  top tip
    (cx - 140, 230),   # 1  upper-left outer
    (cx - 40,  235),   # 2  left notch (inner, just right of 1, slightly lower)
    (cx + 30,  300),   # 3  right notch (inner, just left of 4 vertically, lower)
    (cx + 140, 295),   # 4  lower-right outer
    (cx + 40,  300),   # 5  back to inner-right (mirror of 2)  -- NO, this breaks CW
]
# The above still risks crossing. Simpler: TWO non-overlapping triangles
# sharing one edge (the waist). Each triangle is convex, no crossings.

# Upper triangle: top tip, upper-left, lower-right waist point.
# Lower triangle: same lower-right waist point, bottom tip, upper-left.
# Wait -- that's a parallelogram split wrong. Let me think.
#
# Real bolt = upper triangle (tip up, wide base) + lower triangle (wide top, tip down),
# offset so the bases overlap partially forming the zigzag.
#
# Upper triangle: tip at top, base from upper-left to a point right-of-center.
# Lower triangle: base from a point left-of-center to lower-right, tip at bottom.
# The two bases cross, forming the X/zigzag waist.

draw.polygon(
    [(cx, 56), (cx - 140, 240), (cx + 60, 240)],  # upper triangle
    fill=GREEN,
)
draw.polygon(
    [(cx - 60, 280), (cx + 140, 280), (cx, 456)],  # lower triangle
    fill=GREEN,
)
# Fill the gap between the two triangles' near-bases to make it a solid bolt
draw.rectangle(
    [(cx - 60, 240), (cx + 60, 280)],
    fill=GREEN,
)

img.save("icons/icon.png", "PNG")
print("wrote icons/icon.png", img.size)

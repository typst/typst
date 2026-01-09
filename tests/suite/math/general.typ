// Test general expressions in multiple fonts.

--- math-general-lorenz paged ---
// The Lorenz equations.
$
  dot(x) & = sigma (y - x) \
  dot(y) & = rho x - y - x z \
  dot(z) & = -beta z + x y
$

--- math-general-lorenz-asana paged ---
// The Lorenz equations.
#show math.equation: set text(font: "Asana Math", fallback: false)
$
  dot(x) & = sigma (y - x) \
  dot(y) & = rho x - y - x z \
  dot(z) & = -beta z + x y
$

--- math-general-lorenz-concrete paged ---
// The Lorenz equations.
#show math.equation: set text(font: "Concrete Math", fallback: false)
$
  dot(x) & = sigma (y - x) \
  dot(y) & = rho x - y - x z \
  dot(z) & = -beta z + x y
$

--- math-general-lorenz-garamond paged ---
// The Lorenz equations.
#show math.equation: set text(font: "Garamond-Math", fallback: false)
$
  dot(x) & = sigma (y - x) \
  dot(y) & = rho x - y - x z \
  dot(z) & = -beta z + x y
$

--- math-general-lorenz-ibm-plex paged ---
// The Lorenz equations.
#show math.equation: set text(font: "IBM Plex Math", fallback: false)
$
  dot(x) & = sigma (y - x) \
  dot(y) & = rho x - y - x z \
  dot(z) & = -beta z + x y
$

--- math-general-lorenz-libertinus paged ---
// The Lorenz equations.
#show math.equation: set text(font: "Libertinus Math", fallback: false)
$
  dot(x) & = sigma (y - x) \
  dot(y) & = rho x - y - x z \
  dot(z) & = -beta z + x y
$

--- math-general-lorenz-noto-sans paged ---
// The Lorenz equations.
#show math.equation: set text(font: "Noto Sans Math", fallback: false)
$
  dot(x) & = sigma (y - x) \
  dot(y) & = rho x - y - x z \
  dot(z) & = -beta z + x y
$

--- math-general-lorenz-pennstander paged ---
// The Lorenz equations.
#show math.equation: set text(font: "Pennstander Math", fallback: false)
$
  dot(x) & = sigma (y - x) \
  dot(y) & = rho x - y - x z \
  dot(z) & = -beta z + x y
$

--- math-general-lorenz-stix-two paged ---
// The Lorenz equations.
#show math.equation: set text(font: "STIX Two Math", fallback: false)
$
  dot(x) & = sigma (y - x) \
  dot(y) & = rho x - y - x z \
  dot(z) & = -beta z + x y
$

--- math-general-lorenz-tex-gyre-bonum paged ---
// The Lorenz equations.
#show math.equation: set text(font: "TeX Gyre Bonum Math", fallback: false)
$
  dot(x) & = sigma (y - x) \
  dot(y) & = rho x - y - x z \
  dot(z) & = -beta z + x y
$

--- math-general-lorenz-xits paged ---
// The Lorenz equations.
#show math.equation: set text(font: "XITS Math", fallback: false)
$
  dot(x) & = sigma (y - x) \
  dot(y) & = rho x - y - x z \
  dot(z) & = -beta z + x y
$

--- math-general-rogers-ramanujan paged ---
// A Rogers-Ramanujan identity.
#set page(width: auto)
$
  1 + q^2 / ((1 - q)) + q^6 / ((1 - q) (1 - q^2)) + dots.c = product_(j = 0)^oo 1 / ((1 - q^(5j + 2)) (1 - q^(5j + 3))), wide "for" quad abs(q) < 1.
$

--- math-general-rogers-ramanujan-asana paged ---
// A Rogers-Ramanujan identity.
#set page(width: auto)
#show math.equation: set text(font: "Asana Math", fallback: false)
$
  1 + q^2 / ((1 - q)) + q^6 / ((1 - q) (1 - q^2)) + dots.c = product_(j = 0)^oo 1 / ((1 - q^(5j + 2)) (1 - q^(5j + 3))), wide "for" quad abs(q) < 1.
$

--- math-general-rogers-ramanujan-concrete paged ---
// A Rogers-Ramanujan identity.
#set page(width: auto)
#show math.equation: set text(font: "Concrete Math", fallback: false)
$
  1 + q^2 / ((1 - q)) + q^6 / ((1 - q) (1 - q^2)) + dots.c = product_(j = 0)^oo 1 / ((1 - q^(5j + 2)) (1 - q^(5j + 3))), wide "for" quad abs(q) < 1.
$

--- math-general-rogers-ramanujan-garamond paged ---
// A Rogers-Ramanujan identity.
#set page(width: auto)
#show math.equation: set text(font: "Garamond-Math", fallback: false)
$
  1 + q^2 / ((1 - q)) + q^6 / ((1 - q) (1 - q^2)) + dots.c = product_(j = 0)^oo 1 / ((1 - q^(5j + 2)) (1 - q^(5j + 3))), wide "for" quad abs(q) < 1.
$

--- math-general-rogers-ramanujan-ibm-plex paged ---
// A Rogers-Ramanujan identity.
#set page(width: auto)
#show math.equation: set text(font: "IBM Plex Math", fallback: false)
$
  1 + q^2 / ((1 - q)) + q^6 / ((1 - q) (1 - q^2)) + dots.c = product_(j = 0)^oo 1 / ((1 - q^(5j + 2)) (1 - q^(5j + 3))), wide "for" quad abs(q) < 1.
$

--- math-general-rogers-ramanujan-libertinus paged ---
// A Rogers-Ramanujan identity.
#set page(width: auto)
#show math.equation: set text(font: "Libertinus Math", fallback: false)
$
  1 + q^2 / ((1 - q)) + q^6 / ((1 - q) (1 - q^2)) + dots.c = product_(j = 0)^oo 1 / ((1 - q^(5j + 2)) (1 - q^(5j + 3))), wide "for" quad abs(q) < 1.
$

--- math-general-rogers-ramanujan-noto-sans paged ---
// A Rogers-Ramanujan identity.
#set page(width: auto)
#show math.equation: set text(font: "Noto Sans Math", fallback: false)
$
  1 + q^2 / ((1 - q)) + q^6 / ((1 - q) (1 - q^2)) + dots.c = product_(j = 0)^oo 1 / ((1 - q^(5j + 2)) (1 - q^(5j + 3))), wide "for" quad abs(q) < 1.
$

--- math-general-rogers-ramanujan-pennstander paged ---
// A Rogers-Ramanujan identity.
#set page(width: auto)
#show math.equation: set text(font: "Pennstander Math", fallback: false)
$
  1 + q^2 / ((1 - q)) + q^6 / ((1 - q) (1 - q^2)) + dots.c = product_(j = 0)^oo 1 / ((1 - q^(5j + 2)) (1 - q^(5j + 3))), wide "for" quad abs(q) < 1.
$

--- math-general-rogers-ramanujan-stix-two paged ---
// A Rogers-Ramanujan identity.
#set page(width: auto)
#show math.equation: set text(font: "STIX Two Math", fallback: false)
$
  1 + q^2 / ((1 - q)) + q^6 / ((1 - q) (1 - q^2)) + dots.c = product_(j = 0)^oo 1 / ((1 - q^(5j + 2)) (1 - q^(5j + 3))), wide "for" quad abs(q) < 1.
$

--- math-general-rogers-ramanujan-tex-gyre-bonum paged ---
// A Rogers-Ramanujan identity.
#set page(width: auto)
#show math.equation: set text(font: "TeX Gyre Bonum Math", fallback: false)
$
  1 + q^2 / ((1 - q)) + q^6 / ((1 - q) (1 - q^2)) + dots.c = product_(j = 0)^oo 1 / ((1 - q^(5j + 2)) (1 - q^(5j + 3))), wide "for" quad abs(q) < 1.
$

--- math-general-rogers-ramanujan-xits paged ---
// A Rogers-Ramanujan identity.
#set page(width: auto)
#show math.equation: set text(font: "XITS Math", fallback: false)
$
  1 + q^2 / ((1 - q)) + q^6 / ((1 - q) (1 - q^2)) + dots.c = product_(j = 0)^oo 1 / ((1 - q^(5j + 2)) (1 - q^(5j + 3))), wide "for" quad abs(q) < 1.
$

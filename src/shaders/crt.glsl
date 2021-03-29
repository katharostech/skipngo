const float PI = 3.14159265359;

in vec2 uv;

out vec4 frag_color;

uniform uvec2 window_size;
uniform float time;
uniform sampler2D screen_texture;

const vec2 screen_size = vec2(1280.0, 1024.0);
const bool show_curvature = true;
const float curvature_x_amount  = float(6.0); 
const float curvature_y_amount  = float(4.0);
const vec4 corner_color = vec4(0.0, 0.0, 0.0, 1.0);
const bool show_vignette = true;
const float vignette_opacity = 0.2;
const bool show_horizontal_scan_lines = true;
const float horizontal_scan_lines_amount = 180.0;
const float horizontal_scan_lines_opacity = 0.2;
const bool show_vertical_scan_lines = false;
const float vertical_scan_lines_amount = 370.0;
const float vertical_scan_lines_opacity = 1.0;
const float horizontal_scan_lines_velocity = 0.005;
const float boost = 1.2;
const float aberration_amount = 1.0;

vec2 l_uv_curve(vec2 l_uv) {
	if (show_curvature) {
		l_uv = l_uv * 2.0 - 1.0;
		vec2 offset = abs(l_uv.yx) / vec2(curvature_x_amount, curvature_y_amount);
		l_uv = l_uv + l_uv * offset * offset;
		l_uv = l_uv * 0.5 + 0.5;
	}

	return l_uv;
}


void main() {
	vec2 UV = uv;
	vec2 l_uv = l_uv_curve(UV);
	vec2 screen_l_uv = l_uv_curve(uv);
	vec3 color = texture(screen_texture, screen_l_uv).rgb;

	if (aberration_amount > 0.0) {
		float adjusted_amount = aberration_amount / screen_size.x;
		color.r = texture(screen_texture, vec2(screen_l_uv.x + adjusted_amount, screen_l_uv.y)).r;
		color.g = texture(screen_texture, screen_l_uv).g;
		color.b = texture(screen_texture, vec2(screen_l_uv.x - adjusted_amount, screen_l_uv.y)).b;
	}

	if (show_vignette) {
		float vignette = l_uv.x * l_uv.y * (1.0 - l_uv.x) * (1.0 - l_uv.y);
		vignette = clamp(pow((screen_size.x / 4.0) * vignette, vignette_opacity), 0.0, 1.0);
		color *= vignette;
	}

	if (show_horizontal_scan_lines) {
		float s = sin((screen_l_uv.y + time * horizontal_scan_lines_velocity) * horizontal_scan_lines_amount * PI * 2.0);
		s = (s * 0.5 + 0.5) * 0.9 + 0.1;
		vec4 scan_line = vec4(vec3(pow(s, horizontal_scan_lines_opacity)), 1.0);
		color *= scan_line.rgb;
	}

	if (show_vertical_scan_lines) {
		float s = sin(screen_l_uv.x * vertical_scan_lines_amount * PI * 2.0);
		s = (s * 0.5 + 0.5) * 0.9 + 0.1;
		vec4 scan_line = vec4(vec3(pow(s, vertical_scan_lines_opacity)), 1.0);
		color *= scan_line.rgb;
	}

	if (show_horizontal_scan_lines || show_vertical_scan_lines) {
		color *= boost;
	}

	// Fill the blank space of the corners, left by the curvature, with black.
	if (l_uv.x < 0.0 || l_uv.x > 1.0 || l_uv.y < 0.0 || l_uv.y > 1.0) {
		color = corner_color.rgb;
	}

	frag_color = vec4(color, 1.0);
}
// ----- config -----

const GRID_W      = 500;
const GRID_H      = 240;
const COUNT       = 160;
const FLAKE_SIZE  = 1;
const ANGLE       = (5 * Math.PI) / 4;
const DIR_X       = Math.cos(ANGLE);
const DIR_Y       = -Math.sin(ANGLE);

// ----- types -----

interface Flake {
    x:       number;
    y:       number;
    px:      number;
    py:      number;
    speed:   number;
    opacity: number;
}

// ----- factory -----

function mkFlake(fromTop = false): Flake {
    const x = Math.random() * (GRID_W + GRID_H);
    const y = fromTop ? -2 : Math.random() * GRID_H;
    return {
        x,
        y,
        px:      Math.floor(x),
        py:      Math.floor(y),
        speed:   0.5 + Math.random() * 0.6,
        opacity: 0.45 + Math.random() * 0.55,
    };
}

// ----- draw plus -----

function drawPlus(ctx: CanvasRenderingContext2D, x: number, y: number) {
    const s = FLAKE_SIZE;
    ctx.fillRect(x,         y - s,     s,     s * 3);
    ctx.fillRect(x - s,     y,         s * 3, s);
}

// ----- core -----

function runSnow(canvas: HTMLCanvasElement): () => void {
    const ctx  = canvas.getContext('2d')!;
    let raf: number;
    let alive  = true;

    canvas.width  = GRID_W;
    canvas.height = GRID_H;

    const flakes: Flake[] = Array.from({ length: COUNT }, () => mkFlake());

    const tick = () => {
        if (!alive) return;

        ctx.clearRect(0, 0, GRID_W, GRID_H);

        for (const f of flakes) {
            f.x += DIR_X * f.speed;
            f.y += DIR_Y * f.speed;

            const nx = Math.floor(f.x);
            const ny = Math.floor(f.y);

            if (nx !== f.px || ny !== f.py) {
                f.px = nx;
                f.py = ny;
            }

            if (f.py > GRID_H + 2 || f.px < -2) {
                Object.assign(f, mkFlake(true));
                continue;
            }

            ctx.fillStyle = `rgba(255,255,255,${f.opacity.toFixed(2)})`;
            drawPlus(ctx, f.px, f.py);
        }

        raf = requestAnimationFrame(tick);
    };

    raf = requestAnimationFrame(tick);

    return () => {
        alive = false;
        cancelAnimationFrame(raf);
    };
}

// ----- public api -----

const registry = new Map<string, () => void>();

export function attachSnow(containerId: string): void {
    detachSnow(containerId);

    const container = document.getElementById(containerId);
    if (!container) return;

    const existing = container.style.position;
    if (!existing || existing === 'static') {
        container.style.position = 'relative';
    }

    const canvas = document.createElement('canvas');
    canvas.dataset.snow = containerId;
    canvas.style.cssText = [
        'position:absolute',
        'inset:0',
        'width:100%',
        'height:100%',
        'pointer-events:none',
        'image-rendering:pixelated',
        'image-rendering:crisp-edges',
        'z-index:0',
    ].join(';');

    container.prepend(canvas);

    const stop = runSnow(canvas);

    registry.set(containerId, () => {
        stop();
        canvas.remove();
    });
}

export function detachSnow(containerId: string): void {
    registry.get(containerId)?.();
    registry.delete(containerId);
}

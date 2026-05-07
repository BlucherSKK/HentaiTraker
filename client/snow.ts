// ----- config -----

const GRID_W  = 240;
const GRID_H  = 130;
const COUNT   = 80;

// ----- types -----

interface Flake {
    x:       number;
    y:       number;
    size:    1 | 2;
    speed:   number;
    drift:   number;
    opacity: number;
}

// ----- factory -----

function mkFlake(fromTop = false): Flake {
    return {
        x:       Math.random() * GRID_W,
        y:       fromTop ? -2 : Math.random() * GRID_H,
        size:    Math.random() < 0.3 ? 2 : 1,
        speed:   0.15 + Math.random() * 0.45,
        drift:   (Math.random() - 0.5) * 0.12,
        opacity: 0.5 + Math.random() * 0.5,
    };
}

// ----- core -----

function runSnow(canvas: HTMLCanvasElement): () => void {
    const ctx    = canvas.getContext('2d')!;
    let raf: number;
    let alive    = true;

    canvas.width  = GRID_W;
    canvas.height = GRID_H;

    const flakes: Flake[] = Array.from({ length: COUNT }, () => mkFlake());

    const tick = () => {
        if (!alive) return;

        ctx.clearRect(0, 0, GRID_W, GRID_H);

        for (const f of flakes) {
            f.y += f.speed;
            f.x += f.drift;

            if (f.y > GRID_H + f.size) { Object.assign(f, mkFlake(true)); }
            if (f.x >  GRID_W + f.size) { f.x = -f.size; }
            if (f.x < -f.size)          { f.x =  GRID_W + f.size; }

            const px = Math.round(f.x);
            const py = Math.round(f.y);

            ctx.fillStyle = `rgba(255,255,255,${f.opacity.toFixed(2)})`;
            ctx.fillRect(px, py, f.size, f.size);
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

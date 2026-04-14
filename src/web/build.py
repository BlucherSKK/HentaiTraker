import esbuild

esbuild.build(
    entry_points=['app.js'],
    bundle=True,
    minify=True,
    outfile='app.min.js',
)

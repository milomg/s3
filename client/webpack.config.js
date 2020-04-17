const path = require("path");
const CopyPlugin = require("copy-webpack-plugin");
const TerserPlugin = require("terser-webpack-plugin");

module.exports = {
  entry: "./src/index.ts",
  mode: "development",
  devtool: "cheap-source-map",
  module: {
    rules: [
      {
        test: /\.tsx?$/,
        use: "ts-loader",
        exclude: /node_modules/,
      },
    ],
  },
  plugins: [new CopyPlugin(["assets"])],
  resolve: {
    extensions: [".tsx", ".ts", ".js"],
  },
  output: {
    filename: "[name].js",
    path: path.resolve(__dirname, "dist"),
    pathinfo: false,
  },
  optimization: {
    minimize: true,
    minimizer: [
      new TerserPlugin({
        cache: true,
        chunkFilter: (chunk) => {
          // Only uglify the `vendor` chunk
          return chunk.name === "vendor";
        },
        sourceMap: true,
      }),
    ],
    runtimeChunk: "single",
    namedChunks: false,
    namedModules: false,
    splitChunks: {
      hidePathInfo: true,
      cacheGroups: {
        vendor: {
          test: /[\\/]node_modules[\\/]/,
          chunks: "all",
          name: "vendor",
        },
      },
    },
  },
};

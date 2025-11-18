#!/usr/bin/env python3
import pandas as pd
import sys
import argparse


def report(df, group_by_cols, numeric_cols):
    if not group_by_cols:
        results = {}
        for col in numeric_cols:
            mean = df[col].mean()
            std = df[col].std(ddof=0)
            cv = (std / mean * 100) if mean != 0 else 0
            results[col] = {"mean": mean, "cv": cv}

        output_df = pd.DataFrame(results).T
        output_df.columns = ["mean", "cv_%"]
        return output_df

    grouped = df.groupby(group_by_cols)

    results = []
    for name, group in grouped:
        row = {}
        if isinstance(name, tuple):
            for i, col in enumerate(group_by_cols):
                row[col] = name[i]
        else:
            row[group_by_cols[0]] = name

        for col in numeric_cols:
            mean = group[col].mean()
            std = group[col].std(ddof=0)
            cv = (std / mean * 100) if mean != 0 else 0

            row[f"{col}_mean"] = mean
            row[f"{col}_cv%"] = cv

        results.append(row)

    return pd.DataFrame(results)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("input_file", help="Input CSV file")
    parser.add_argument(
        "-g", "--group-by", nargs="+", default=[], help="Column names to group by"
    )

    args = parser.parse_args()

    try:
        df = pd.read_csv(args.input_file)
    except Exception as e:
        print(f"Error reading file: {e}", file=sys.stderr)
        sys.exit(1)

    for col in args.group_by:
        if col not in df.columns:
            print(f"Error: Column '{col}' not found in CSV", file=sys.stderr)
            sys.exit(1)

    ignore_cols = ["test", "nice", "affinity", "iteration", "ended"]
    numeric_cols = df.select_dtypes(include=["number"]).columns.tolist()
    numeric_cols = [
        col
        for col in numeric_cols
        if col not in args.group_by and col not in ignore_cols
    ]

    if not numeric_cols:
        print("Error: No numeric columns found", file=sys.stderr)
        sys.exit(1)

    result_df = report(df, args.group_by, numeric_cols)
    result_df = result_df.fillna(0)
    result_df = result_df.round(3)
    print(result_df.to_csv(index=False))


if __name__ == "__main__":
    main()

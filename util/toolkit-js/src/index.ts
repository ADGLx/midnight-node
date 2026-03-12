// This file is part of midnight-node.
// Copyright (C) Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0 (the "License");
// You may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import {Effect, Layer, Logger, LogLevel} from 'effect';
import {Command, CliConfig, Options} from '@effect/cli';
import {NodeContext, NodeRuntime} from "@effect/platform-node";
import Package from '@midnight-ntwrk/node-toolkit/package.json' with {type: 'json'};
import {getDefaultVersion, getSupportedVersions} from './version-registry.js';

// Pre-parse --compact-version from argv before Effect CLI runs,
// since the value determines which commands to dynamically import.
function parseCompactVersion(argv: string[]): string {
    const idx = argv.indexOf('--compact-version');
    if (idx !== -1 && idx + 1 < argv.length) {
        return argv[idx + 1];
    }
    return getDefaultVersion();
}

async function loadCommands(version: string) {
    switch (version) {
        case '0.29.0': {
            const {deployCommand, circuitCommand, maintainCommand, ConfigCompiler} =
                await import('@midnight-ntwrk/compact-js-command/effect');
            return {
                subcommands: [deployCommand, circuitCommand, maintainCommand] as const,
                configCompilerLayer: ConfigCompiler.layer,
            };
        }
        case '0.30.0-rc.0': {
            const {deployCommand, circuitCommand, maintainCommand, ConfigCompiler} =
                await import('@midnight-ntwrk/compact-js-command-v2-5-0/effect');
            return {
                subcommands: [deployCommand, circuitCommand, maintainCommand] as const,
                configCompilerLayer: ConfigCompiler.layer,
            };
        }
        default:
            throw new Error(
                `Unsupported compact version: ${version}. Supported versions: ${getSupportedVersions().join(', ')}`
            );
    }
}

const compactVersion = parseCompactVersion(process.argv);
const {subcommands, configCompilerLayer} = await loadCommands(compactVersion);

const compactVersionOption = Options.text('compact-version').pipe(
    Options.withDescription(`CompactC compiler version (supported: ${getSupportedVersions().join(', ')})`),
    Options.withDefault(getDefaultVersion()),
);

const cli = Command.run(
    Command.make('midnight-node-toolkit-js', {compactVersion: compactVersionOption}).pipe(
        Command.withDescription('Provides utilities to execute Compact compiled contracts from the command line.'),
        Command.withSubcommands([...subcommands])
    ),
    {
        name: 'Midnight Node Toolkit',
        version: `v${Package.version}`,
        executable: 'midnight-node-toolkit-js'
    }
);

// configCompilerLayer is a union of version-specific Layer types that are
// structurally compatible but nominally distinct; cast to unify them.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const program = cli(process.argv).pipe(
    Logger.withMinimumLogLevel(LogLevel.None),
    Effect.provide(Layer.mergeAll(
        (configCompilerLayer as any).pipe(Layer.provideMerge(NodeContext.layer)),
        CliConfig.layer({showBuiltIns: false})
    )),
) as Effect.Effect<void>;

program.pipe(NodeRuntime.runMain({disableErrorReporting: false}));

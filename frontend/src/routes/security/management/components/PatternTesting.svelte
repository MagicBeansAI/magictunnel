<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	
	export let securityState;
	
	const dispatch = createEventDispatcher();
	
	// Component state
	let testMode = 'single'; // 'single' or 'batch'
	let singlePattern = {
		pattern: '',
		pattern_type: 'regex',
		test_input: '',
		priority: 100
	};
	
	let batchPatterns = '';
	let batchTestInputs = '';
	
	let testResults = [];
	let loading = false;
	let error = null;
	
	// Pattern examples for quick testing
	const patternExamples = [
		{
			name: 'Destructive Operations',
			pattern: '.*(?:delete|destroy|remove|rm|kill|terminate).*',
			type: 'regex',
			description: 'Matches tools with destructive operations'
		},
		{
			name: 'File Operations',
			pattern: 'file_.*',
			type: 'regex',
			description: 'Matches all file-related tools'
		},
		{
			name: 'Database Operations',
			pattern: '(?:db|database)_.*',
			type: 'regex',
			description: 'Matches database-related tools'
		},
		{
			name: 'System Commands',
			pattern: 'system_.*|cmd_.*|exec_.*',
			type: 'regex',
			description: 'Matches system command tools'
		},
		{
			name: 'Network Operations',
			pattern: '(?:net|network|http|ftp|ssh)_.*',
			type: 'regex',
			description: 'Matches network-related tools'
		}
	];
	
	// Test input examples
	const testInputExamples = [
		'file_delete_user_data',
		'database_backup_create',
		'system_process_kill',
		'network_ping_host',
		'file_copy_documents',
		'db_restore_backup',
		'exec_shell_command',
		'http_fetch_data',
		'user_create_account',
		'email_send_notification'
	];
	
	function setTestMode(mode) {
		testMode = mode;
		clearResults();
	}
	
	function clearResults() {
		testResults = [];
		error = null;
	}
	
	function loadPatternExample(example) {
		singlePattern.pattern = example.pattern;
		singlePattern.pattern_type = example.type;
		clearResults();
	}
	
	function addTestInput(input) {
		if (testMode === 'single') {
			singlePattern.test_input = input;
		} else {
			const currentInputs = batchTestInputs.split('\n').filter(line => line.trim());
			if (!currentInputs.includes(input)) {
				batchTestInputs = [...currentInputs, input].join('\n');
			}
		}
	}
	
	async function testSinglePattern() {
		if (!singlePattern.pattern.trim()) {
			error = 'Pattern is required';
			return;
		}
		
		if (!singlePattern.test_input.trim()) {
			error = 'Test input is required';
			return;
		}
		
		try {
			loading = true;
			error = null;
			
			const response = await fetch('/api/security/patterns/test', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					patterns: [{
						pattern: singlePattern.pattern,
						pattern_type: singlePattern.pattern_type,
						priority: singlePattern.priority
					}],
					test_inputs: [singlePattern.test_input]
				})
			});
			
			if (!response.ok) {
				const errorData = await response.json();
				throw new Error(errorData.error || 'Pattern test failed');
			}
			
			const data = await response.json();
			testResults = data.data.results || [];
			
		} catch (err) {
			error = err.message;
			console.error('Failed to test pattern:', err);
		} finally {
			loading = false;
		}
	}
	
	async function testBatchPatterns() {
		const patterns = batchPatterns.split('\n')
			.map(line => line.trim())
			.filter(line => line && !line.startsWith('#'))
			.map((pattern, index) => ({
				pattern,
				pattern_type: 'regex',
				priority: (index + 1) * 10
			}));
		
		const testInputs = batchTestInputs.split('\n')
			.map(line => line.trim())
			.filter(line => line);
		
		if (patterns.length === 0) {
			error = 'At least one pattern is required';
			return;
		}
		
		if (testInputs.length === 0) {
			error = 'At least one test input is required';
			return;
		}
		
		if (patterns.length > 50) {
			error = 'Maximum 50 patterns allowed for batch testing';
			return;
		}
		
		try {
			loading = true;
			error = null;
			
			const response = await fetch('/api/security/patterns/test', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					patterns,
					test_inputs: testInputs
				})
			});
			
			if (!response.ok) {
				const errorData = await response.json();
				throw new Error(errorData.error || 'Batch pattern test failed');
			}
			
			const data = await response.json();
			testResults = data.data.results || [];
			
		} catch (err) {
			error = err.message;
			console.error('Failed to test batch patterns:', err);
		} finally {
			loading = false;
		}
	}
	
	async function validatePattern() {
		if (!singlePattern.pattern.trim()) {
			error = 'Pattern is required';
			return;
		}
		
		try {
			loading = true;
			error = null;
			
			const response = await fetch('/api/security/patterns/validate', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					pattern: singlePattern.pattern,
					pattern_type: singlePattern.pattern_type
				})
			});
			
			if (!response.ok) {
				const errorData = await response.json();
				throw new Error(errorData.error || 'Pattern validation failed');
			}
			
			const data = await response.json();
			
			if (data.data.is_valid) {
				testResults = [{
					pattern: singlePattern.pattern,
					validation: {
						is_valid: true,
						compiled_successfully: true,
						message: 'Pattern is valid and compiles successfully'
					}
				}];
			} else {
				error = data.data.error || 'Pattern is invalid';
			}
			
		} catch (err) {
			error = err.message;
			console.error('Failed to validate pattern:', err);
		} finally {
			loading = false;
		}
	}
	
	function getMatchBadgeClass(matched) {
		return matched ? 'match-success' : 'match-failure';
	}
	
	function getMatchIcon(matched) {
		return matched ? '✅' : '❌';
	}
	
	function formatExecutionTime(time) {
		if (time < 1000) {
			return `${Math.round(time)}ns`;
		} else if (time < 1000000) {
			return `${Math.round(time / 1000)}μs`;
		} else {
			return `${Math.round(time / 1000000)}ms`;
		}
	}
</script>

<div class="pattern-testing">
	<div class="header">
		<h2>🧪 Pattern Testing Interface</h2>
		<p>Test and validate regex patterns for the security allowlist system</p>
	</div>

	<!-- Test Mode Selection -->
	<div class="mode-selection">
		<div class="mode-tabs">
			<button 
				class="tab {testMode === 'single' ? 'active' : ''}"
				on:click={() => setTestMode('single')}
			>
				🎯 Single Pattern Test
			</button>
			<button 
				class="tab {testMode === 'batch' ? 'active' : ''}"
				on:click={() => setTestMode('batch')}
			>
				📊 Batch Pattern Test
			</button>
		</div>
	</div>

	{#if testMode === 'single'}
		<!-- Single Pattern Testing -->
		<div class="test-panel">
			<div class="panel-grid">
				<!-- Pattern Input -->
				<div class="input-section">
					<h3>🔍 Pattern Configuration</h3>
					
					<div class="form-group">
						<label>Pattern Type:</label>
						<select bind:value={singlePattern.pattern_type}>
							<option value="regex">Regular Expression</option>
							<option value="wildcard">Wildcard Pattern</option>
						</select>
					</div>
					
					<div class="form-group">
						<label>Pattern:</label>
						<textarea 
							bind:value={singlePattern.pattern}
							placeholder="Enter your regex pattern here..."
							rows="3"
						></textarea>
					</div>
					
					<div class="form-group">
						<label>Priority:</label>
						<input 
							type="number" 
							bind:value={singlePattern.priority}
							min="1"
							max="1000"
							placeholder="100"
						/>
						<small>Lower numbers = higher priority</small>
					</div>
					
					<div class="form-group">
						<label>Test Input:</label>
						<input 
							type="text" 
							bind:value={singlePattern.test_input}
							placeholder="tool_name_to_test"
						/>
					</div>
					
					<div class="actions">
						<button class="btn btn-secondary" on:click={validatePattern} disabled={loading}>
							{loading ? '🔄 Validating...' : '✅ Validate Pattern'}
						</button>
						<button class="btn btn-primary" on:click={testSinglePattern} disabled={loading}>
							{loading ? '🔄 Testing...' : '🧪 Test Pattern'}
						</button>
					</div>
				</div>

				<!-- Examples and Help -->
				<div class="examples-section">
					<h3>📚 Pattern Examples</h3>
					<div class="examples-list">
						{#each patternExamples as example}
							<div class="example-item">
								<div class="example-header">
									<strong>{example.name}</strong>
									<button 
										class="btn-small"
										on:click={() => loadPatternExample(example)}
									>
										Use
									</button>
								</div>
								<div class="example-pattern">
									<code>{example.pattern}</code>
								</div>
								<div class="example-description">
									{example.description}
								</div>
							</div>
						{/each}
					</div>
					
					<h3>🎯 Test Inputs</h3>
					<div class="test-inputs">
						{#each testInputExamples as input}
							<button 
								class="input-tag"
								on:click={() => addTestInput(input)}
							>
								{input}
							</button>
						{/each}
					</div>
				</div>
			</div>
		</div>
	{:else}
		<!-- Batch Pattern Testing -->
		<div class="test-panel">
			<div class="batch-grid">
				<div class="batch-section">
					<h3>📝 Patterns (one per line)</h3>
					<textarea 
						bind:value={batchPatterns}
						placeholder=".*delete.*&#10;file_.*&#10;(?:db|database)_.*&#10;# Comments start with #"
						rows="10"
					></textarea>
					<small>Enter one regex pattern per line. Lines starting with # are ignored.</small>
				</div>
				
				<div class="batch-section">
					<h3>🎯 Test Inputs (one per line)</h3>
					<textarea 
						bind:value={batchTestInputs}
						placeholder="file_delete_user_data&#10;database_backup_create&#10;system_process_kill"
						rows="10"
					></textarea>
					<small>Enter one tool name per line to test against all patterns.</small>
					
					<div class="quick-inputs">
						<strong>Quick add:</strong>
						{#each testInputExamples.slice(0, 5) as input}
							<button 
								class="input-tag"
								on:click={() => addTestInput(input)}
							>
								{input}
							</button>
						{/each}
					</div>
				</div>
			</div>
			
			<div class="batch-actions">
				<button class="btn btn-primary" on:click={testBatchPatterns} disabled={loading}>
					{loading ? '🔄 Testing...' : '🧪 Test All Patterns'}
				</button>
				<button class="btn btn-secondary" on:click={clearResults}>
					🗑️ Clear Results
				</button>
			</div>
		</div>
	{/if}

	<!-- Error Display -->
	{#if error}
		<div class="error-banner">
			⚠️ <strong>Error:</strong> {error}
		</div>
	{/if}

	<!-- Test Results -->
	{#if testResults.length > 0}
		<div class="results-panel">
			<h3>📋 Test Results</h3>
			
			{#each testResults as result}
				<div class="result-item">
					<div class="result-header">
						<h4>Pattern: <code>{result.pattern}</code></h4>
						{#if result.validation}
							<div class="validation-status">
								{#if result.validation.is_valid}
									<span class="status-badge success">✅ Valid</span>
								{:else}
									<span class="status-badge error">❌ Invalid</span>
								{/if}
							</div>
						{/if}
					</div>
					
					{#if result.validation}
						<div class="validation-details">
							<p><strong>Validation:</strong> {result.validation.message}</p>
							{#if result.validation.compilation_time}
								<p><strong>Compilation Time:</strong> {formatExecutionTime(result.validation.compilation_time)}</p>
							{/if}
						</div>
					{/if}
					
					{#if result.matches}
						<div class="matches-section">
							<h5>🎯 Match Results:</h5>
							<div class="matches-grid">
								{#each result.matches as match}
									<div class="match-item">
										<div class="match-input">
											<span class="input-text">{match.input}</span>
											<span class="match-badge {getMatchBadgeClass(match.matched)}">
												{getMatchIcon(match.matched)}
											</span>
										</div>
										{#if match.execution_time}
											<div class="match-time">
												⏱️ {formatExecutionTime(match.execution_time)}
											</div>
										{/if}
									</div>
								{/each}
							</div>
						</div>
					{/if}
					
					{#if result.performance}
						<div class="performance-section">
							<h5>⚡ Performance Metrics:</h5>
							<div class="performance-grid">
								<div class="perf-item">
									<span class="perf-label">Total Time:</span>
									<span class="perf-value">{formatExecutionTime(result.performance.total_time)}</span>
								</div>
								<div class="perf-item">
									<span class="perf-label">Average Time:</span>
									<span class="perf-value">{formatExecutionTime(result.performance.average_time)}</span>
								</div>
								<div class="perf-item">
									<span class="perf-label">Matches:</span>
									<span class="perf-value">{result.performance.match_count}</span>
								</div>
								<div class="perf-item">
									<span class="perf-label">Success Rate:</span>
									<span class="perf-value">{Math.round(result.performance.success_rate * 100)}%</span>
								</div>
							</div>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.pattern-testing {
		padding: 20px;
	}

	.header {
		margin-bottom: 30px;
		text-align: center;
	}

	.header h2 {
		color: #1f2937;
		margin-bottom: 8px;
	}

	.header p {
		color: #6b7280;
		font-size: 1.1em;
	}

	.mode-selection {
		margin-bottom: 30px;
	}

	.mode-tabs {
		display: flex;
		border-bottom: 2px solid #e5e7eb;
	}

	.tab {
		padding: 12px 24px;
		background: none;
		border: none;
		border-bottom: 3px solid transparent;
		cursor: pointer;
		font-weight: 500;
		color: #6b7280;
		transition: all 0.2s;
	}

	.tab:hover {
		background: #f9fafb;
		color: #374151;
	}

	.tab.active {
		color: #2563eb;
		border-bottom-color: #2563eb;
		background: #eff6ff;
	}

	.test-panel {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 12px;
		padding: 24px;
		margin-bottom: 20px;
	}

	.panel-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 30px;
	}

	.input-section h3, .examples-section h3 {
		margin: 0 0 20px 0;
		color: #1f2937;
		font-size: 1.2em;
	}

	.form-group {
		margin-bottom: 20px;
	}

	.form-group label {
		display: block;
		margin-bottom: 8px;
		font-weight: 600;
		color: #374151;
	}

	.form-group input, .form-group select, .form-group textarea {
		width: 100%;
		padding: 10px 12px;
		border: 1px solid #d1d5db;
		border-radius: 6px;
		font-size: 1em;
	}

	.form-group textarea {
		resize: vertical;
		font-family: monospace;
	}

	.form-group small {
		display: block;
		margin-top: 5px;
		color: #6b7280;
		font-size: 0.9em;
	}

	.actions {
		display: flex;
		gap: 10px;
	}

	.btn {
		padding: 10px 16px;
		border: none;
		border-radius: 6px;
		cursor: pointer;
		font-weight: 500;
		transition: all 0.2s;
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}

	.btn-primary {
		background: #2563eb;
		color: white;
	}

	.btn-primary:hover:not(:disabled) {
		background: #1d4ed8;
	}

	.btn-secondary {
		background: #6b7280;
		color: white;
	}

	.btn-secondary:hover:not(:disabled) {
		background: #4b5563;
	}

	.btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.examples-list {
		margin-bottom: 25px;
	}

	.example-item {
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 6px;
		padding: 15px;
		margin-bottom: 12px;
	}

	.example-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 8px;
	}

	.btn-small {
		padding: 4px 8px;
		font-size: 0.8em;
		background: #2563eb;
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
	}

	.btn-small:hover {
		background: #1d4ed8;
	}

	.example-pattern {
		margin-bottom: 8px;
	}

	.example-pattern code {
		background: #1f2937;
		color: #f9fafb;
		padding: 4px 8px;
		border-radius: 4px;
		font-family: monospace;
		font-size: 0.9em;
	}

	.example-description {
		color: #6b7280;
		font-size: 0.9em;
		font-style: italic;
	}

	.test-inputs {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
	}

	.input-tag {
		background: #eff6ff;
		color: #2563eb;
		border: 1px solid #bfdbfe;
		padding: 4px 8px;
		border-radius: 4px;
		font-size: 0.9em;
		cursor: pointer;
		transition: all 0.2s;
	}

	.input-tag:hover {
		background: #dbeafe;
		border-color: #93c5fd;
	}

	.batch-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 30px;
		margin-bottom: 20px;
	}

	.batch-section h3 {
		margin: 0 0 15px 0;
		color: #1f2937;
	}

	.batch-section textarea {
		width: 100%;
		padding: 12px;
		border: 1px solid #d1d5db;
		border-radius: 6px;
		font-family: monospace;
		font-size: 0.9em;
		resize: vertical;
	}

	.quick-inputs {
		margin-top: 15px;
	}

	.quick-inputs strong {
		display: block;
		margin-bottom: 8px;
		color: #374151;
	}

	.batch-actions {
		display: flex;
		gap: 10px;
		justify-content: center;
	}

	.error-banner {
		background: #fef2f2;
		color: #dc2626;
		border: 1px solid #fecaca;
		padding: 15px 20px;
		border-radius: 8px;
		margin-bottom: 20px;
	}

	.results-panel {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 12px;
		padding: 24px;
	}

	.results-panel h3 {
		margin: 0 0 20px 0;
		color: #1f2937;
	}

	.result-item {
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 8px;
		padding: 20px;
		margin-bottom: 20px;
	}

	.result-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 15px;
	}

	.result-header h4 {
		margin: 0;
		color: #1f2937;
	}

	.result-header code {
		background: #1f2937;
		color: #f9fafb;
		padding: 4px 8px;
		border-radius: 4px;
		font-family: monospace;
	}

	.status-badge {
		padding: 4px 8px;
		border-radius: 4px;
		font-size: 0.8em;
		font-weight: 600;
	}

	.status-badge.success {
		background: #f0fdf4;
		color: #16a34a;
	}

	.status-badge.error {
		background: #fef2f2;
		color: #dc2626;
	}

	.validation-details {
		margin-bottom: 15px;
		padding: 15px;
		background: #fffbeb;
		border: 1px solid #fde68a;
		border-radius: 6px;
	}

	.matches-section h5, .performance-section h5 {
		margin: 0 0 15px 0;
		color: #374151;
	}

	.matches-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 10px;
		margin-bottom: 15px;
	}

	.match-item {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 6px;
		padding: 12px;
	}

	.match-input {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 8px;
	}

	.input-text {
		font-family: monospace;
		font-size: 0.9em;
		color: #374151;
	}

	.match-badge {
		padding: 2px 6px;
		border-radius: 4px;
		font-size: 0.8em;
		font-weight: 600;
	}

	.match-badge.match-success {
		background: #f0fdf4;
		color: #16a34a;
	}

	.match-badge.match-failure {
		background: #fef2f2;
		color: #dc2626;
	}

	.match-time {
		font-size: 0.8em;
		color: #6b7280;
	}

	.performance-section {
		margin-top: 15px;
		padding-top: 15px;
		border-top: 1px solid #e5e7eb;
	}

	.performance-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
		gap: 15px;
	}

	.perf-item {
		background: white;
		padding: 12px;
		border-radius: 6px;
		border: 1px solid #e5e7eb;
		text-align: center;
	}

	.perf-label {
		display: block;
		font-size: 0.8em;
		color: #6b7280;
		margin-bottom: 5px;
	}

	.perf-value {
		display: block;
		font-weight: bold;
		color: #1f2937;
		font-size: 1.1em;
	}

	/* Responsive design */
	@media (max-width: 768px) {
		.pattern-testing {
			padding: 15px;
		}

		.panel-grid, .batch-grid {
			grid-template-columns: 1fr;
		}

		.tab {
			font-size: 0.9em;
			padding: 10px 16px;
		}

		.actions, .batch-actions {
			flex-direction: column;
		}

		.btn {
			width: 100%;
			justify-content: center;
		}

		.result-header {
			flex-direction: column;
			align-items: flex-start;
			gap: 10px;
		}

		.matches-grid, .performance-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
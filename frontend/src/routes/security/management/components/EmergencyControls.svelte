<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	
	export let securityState;
	
	const dispatch = createEventDispatcher();
	
	let loading = false;
	let error = null;
	let showActivateConfirm = false;
	let showDeactivateConfirm = false;
	let confirmationText = '';
	let lockdownStatistics = null;
	let authorizedUsers = [];
	
	// Load emergency lockdown statistics and configuration
	async function loadEmergencyData() {
		try {
			// Load statistics
			const statsResponse = await fetch('/api/security/emergency/lockdown/statistics');
			if (statsResponse.ok) {
				const statsData = await statsResponse.json();
				lockdownStatistics = statsData.data;
			}
			
			// Load authorized users (would be from config or API)
			authorizedUsers = ['admin', 'security_officer', 'system_admin'];
			
		} catch (err) {
			console.error('Failed to load emergency data:', err);
		}
	}
	
	// Load data on component mount
	loadEmergencyData();
	
	function showActivateDialog() {
		showActivateConfirm = true;
		confirmationText = '';
		error = null;
	}
	
	function showDeactivateDialog() {
		showDeactivateConfirm = true;
		confirmationText = '';
		error = null;
	}
	
	function cancelAction() {
		showActivateConfirm = false;
		showDeactivateConfirm = false;
		confirmationText = '';
		error = null;
	}
	
	async function activateEmergencyLockdown() {
		if (confirmationText !== 'ACTIVATE LOCKDOWN') {
			error = 'Please type "ACTIVATE LOCKDOWN" to confirm';
			return;
		}
		
		try {
			loading = true;
			error = null;
			
			const response = await fetch('/api/security/emergency/lockdown/activate', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					activated_by: 'dashboard_user', // Would get from auth context
					reason: 'Emergency lockdown activated via management dashboard',
					notify_users: true
				})
			});
			
			if (!response.ok) {
				const errorData = await response.json();
				throw new Error(errorData.error || 'Failed to activate emergency lockdown');
			}
			
			// Success - refresh security state and close dialog
			dispatch('refresh');
			await loadEmergencyData();
			cancelAction();
			
		} catch (err) {
			error = err.message;
			console.error('Failed to activate emergency lockdown:', err);
		} finally {
			loading = false;
		}
	}
	
	async function deactivateEmergencyLockdown() {
		if (confirmationText !== 'DEACTIVATE LOCKDOWN') {
			error = 'Please type "DEACTIVATE LOCKDOWN" to confirm';
			return;
		}
		
		try {
			loading = true;
			error = null;
			
			const response = await fetch('/api/security/emergency/lockdown/deactivate', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					deactivated_by: 'dashboard_user', // Would get from auth context
					reason: 'Emergency lockdown deactivated via management dashboard'
				})
			});
			
			if (!response.ok) {
				const errorData = await response.json();
				throw new Error(errorData.error || 'Failed to deactivate emergency lockdown');
			}
			
			// Success - refresh security state and close dialog
			dispatch('refresh');
			await loadEmergencyData();
			cancelAction();
			
		} catch (err) {
			error = err.message;
			console.error('Failed to deactivate emergency lockdown:', err);
		} finally {
			loading = false;
		}
	}
	
	function formatTimestamp(timestamp) {
		if (!timestamp) return 'Never';
		return new Date(timestamp).toLocaleString();
	}
	
	function formatDuration(startTime) {
		if (!startTime) return '0 minutes';
		const start = new Date(startTime);
		const now = new Date();
		const diffMs = now - start;
		const diffMinutes = Math.floor(diffMs / (1000 * 60));
		const diffHours = Math.floor(diffMinutes / 60);
		const diffDays = Math.floor(diffHours / 24);
		
		if (diffDays > 0) {
			return `${diffDays} day${diffDays > 1 ? 's' : ''}, ${diffHours % 24} hour${diffHours % 24 !== 1 ? 's' : ''}`;
		} else if (diffHours > 0) {
			return `${diffHours} hour${diffHours > 1 ? 's' : ''}, ${diffMinutes % 60} minute${diffMinutes % 60 !== 1 ? 's' : ''}`;
		} else {
			return `${diffMinutes} minute${diffMinutes !== 1 ? 's' : ''}`;
		}
	}
</script>

<div class="emergency-controls">
	<div class="header">
		<h2>🚨 Emergency Lockdown Controls</h2>
		<p>Emergency lockdown system provides immediate security protection by blocking all tool requests</p>
	</div>

	<!-- Current Status Card -->
	<div class="status-card {$securityState.emergencyLockdown.isActive ? 'emergency-active' : 'emergency-inactive'}">
		<div class="status-header">
			<h3>
				{#if $securityState.emergencyLockdown.isActive}
					🔴 EMERGENCY LOCKDOWN ACTIVE
				{:else}
					🟢 Normal Operation
				{/if}
			</h3>
			<div class="status-indicator {$securityState.emergencyLockdown.isActive ? 'active' : 'inactive'}">
				{$securityState.emergencyLockdown.isActive ? 'LOCKDOWN' : 'NORMAL'}
			</div>
		</div>
		
		<div class="status-details">
			{#if $securityState.emergencyLockdown.isActive}
				<div class="detail-row">
					<span class="label">Activated By:</span>
					<span class="value">{$securityState.emergencyLockdown.activatedBy || 'Unknown'}</span>
				</div>
				<div class="detail-row">
					<span class="label">Activated At:</span>
					<span class="value">{formatTimestamp($securityState.emergencyLockdown.activatedAt)}</span>
				</div>
				<div class="detail-row">
					<span class="label">Duration:</span>
					<span class="value">{formatDuration($securityState.emergencyLockdown.activatedAt)}</span>
				</div>
				<div class="detail-row">
					<span class="label">Blocked Requests:</span>
					<span class="value blocked-count">{$securityState.emergencyLockdown.blockedRequests}</span>
				</div>
			{:else}
				<div class="detail-row">
					<span class="label">System Status:</span>
					<span class="value">All tool requests are being processed normally</span>
				</div>
				<div class="detail-row">
					<span class="label">Last Lockdown:</span>
					<span class="value">{lockdownStatistics?.last_lockdown_at ? formatTimestamp(lockdownStatistics.last_lockdown_at) : 'Never'}</span>
				</div>
			{/if}
		</div>
	</div>

	<!-- Statistics Cards -->
	{#if lockdownStatistics}
		<div class="statistics-grid">
			<div class="stat-card">
				<h4>📊 Total Lockdowns</h4>
				<div class="stat-value">{lockdownStatistics.total_lockdowns || 0}</div>
				<div class="stat-label">Historical activations</div>
			</div>
			
			<div class="stat-card">
				<h4>⏱️ Total Duration</h4>
				<div class="stat-value">{Math.round((lockdownStatistics.total_lockdown_duration_seconds || 0) / 60)}</div>
				<div class="stat-label">Minutes in lockdown</div>
			</div>
			
			<div class="stat-card">
				<h4>🚫 Blocked Requests</h4>
				<div class="stat-value">{lockdownStatistics.total_blocked_requests || 0}</div>
				<div class="stat-label">Total blocked</div>
			</div>
			
			<div class="stat-card">
				<h4>⚡ Response Time</h4>
				<div class="stat-value">{lockdownStatistics.average_response_time_ms || '< 1'}</div>
				<div class="stat-label">ms average</div>
			</div>
		</div>
	{/if}

	<!-- Control Buttons -->
	<div class="control-section">
		<h3>🎛️ Lockdown Controls</h3>
		<div class="control-buttons">
			{#if $securityState.emergencyLockdown.isActive}
				<button 
					class="btn btn-success" 
					on:click={showDeactivateDialog}
					disabled={loading}
				>
					✅ Deactivate Emergency Lockdown
				</button>
			{:else}
				<button 
					class="btn btn-danger" 
					on:click={showActivateDialog}
					disabled={loading}
				>
					🚨 Activate Emergency Lockdown
				</button>
			{/if}
		</div>
		
		<div class="control-warning">
			<strong>⚠️ Warning:</strong> Emergency lockdown will immediately block all tool requests system-wide. 
			Use only when immediate security protection is required.
		</div>
	</div>

	<!-- Authorized Users -->
	<div class="authorized-users">
		<h3>👥 Authorized Users</h3>
		<div class="user-list">
			{#each authorizedUsers as user}
				<span class="user-badge">{user}</span>
			{/each}
		</div>
		<p class="user-note">Only these users can activate/deactivate emergency lockdown</p>
	</div>
</div>

<!-- Activation Confirmation Modal -->
{#if showActivateConfirm}
	<div class="modal-backdrop" on:click={cancelAction}>
		<div class="modal emergency-modal" on:click|stopPropagation>
			<div class="modal-header">
				<h3>🚨 Activate Emergency Lockdown</h3>
				<button class="close-btn" on:click={cancelAction}>✕</button>
			</div>
			
			<div class="modal-content">
				<div class="warning-box">
					<strong>⚠️ CRITICAL ACTION WARNING</strong>
					<p>This will immediately block ALL tool requests system-wide. Only activate in genuine emergency situations.</p>
				</div>
				
				<div class="confirmation-section">
					<label for="confirm-activate">Type "ACTIVATE LOCKDOWN" to confirm:</label>
					<input 
						id="confirm-activate"
						type="text" 
						bind:value={confirmationText}
						placeholder="ACTIVATE LOCKDOWN"
						class="confirmation-input"
					/>
				</div>
				
				{#if error}
					<div class="error-message">{error}</div>
				{/if}
				
				<div class="modal-actions">
					<button class="btn btn-secondary" on:click={cancelAction} disabled={loading}>
						Cancel
					</button>
					<button 
						class="btn btn-danger" 
						on:click={activateEmergencyLockdown}
						disabled={loading || confirmationText !== 'ACTIVATE LOCKDOWN'}
					>
						{#if loading}
							🔄 Activating...
						{:else}
							🚨 Activate Lockdown
						{/if}
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}

<!-- Deactivation Confirmation Modal -->
{#if showDeactivateConfirm}
	<div class="modal-backdrop" on:click={cancelAction}>
		<div class="modal normal-modal" on:click|stopPropagation>
			<div class="modal-header">
				<h3>✅ Deactivate Emergency Lockdown</h3>
				<button class="close-btn" on:click={cancelAction}>✕</button>
			</div>
			
			<div class="modal-content">
				<div class="info-box">
					<strong>ℹ️ Deactivation Confirmation</strong>
					<p>This will restore normal tool request processing. Ensure the security situation has been resolved.</p>
				</div>
				
				<div class="confirmation-section">
					<label for="confirm-deactivate">Type "DEACTIVATE LOCKDOWN" to confirm:</label>
					<input 
						id="confirm-deactivate"
						type="text" 
						bind:value={confirmationText}
						placeholder="DEACTIVATE LOCKDOWN"
						class="confirmation-input"
					/>
				</div>
				
				{#if error}
					<div class="error-message">{error}</div>
				{/if}
				
				<div class="modal-actions">
					<button class="btn btn-secondary" on:click={cancelAction} disabled={loading}>
						Cancel
					</button>
					<button 
						class="btn btn-success" 
						on:click={deactivateEmergencyLockdown}
						disabled={loading || confirmationText !== 'DEACTIVATE LOCKDOWN'}
					>
						{#if loading}
							🔄 Deactivating...
						{:else}
							✅ Deactivate Lockdown
						{/if}
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.emergency-controls {
		padding: 20px;
	}

	.header {
		margin-bottom: 30px;
		text-align: center;
	}

	.header h2 {
		color: #dc2626;
		margin-bottom: 8px;
	}

	.header p {
		color: #6b7280;
		font-size: 1.1em;
	}

	.status-card {
		background: white;
		border-radius: 12px;
		padding: 24px;
		margin-bottom: 30px;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
		border: 2px solid;
	}

	.status-card.emergency-active {
		border-color: #dc2626;
		background: linear-gradient(135deg, #fef2f2, #ffffff);
	}

	.status-card.emergency-inactive {
		border-color: #16a34a;
		background: linear-gradient(135deg, #f0fdf4, #ffffff);
	}

	.status-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 20px;
	}

	.status-header h3 {
		margin: 0;
		font-size: 1.3em;
	}

	.status-indicator {
		padding: 8px 16px;
		border-radius: 20px;
		font-weight: bold;
		font-size: 0.9em;
		text-transform: uppercase;
	}

	.status-indicator.active {
		background: #dc2626;
		color: white;
		animation: pulse 2s infinite;
	}

	.status-indicator.inactive {
		background: #16a34a;
		color: white;
	}

	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.7; }
	}

	.status-details {
		display: grid;
		gap: 12px;
	}

	.detail-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 10px 0;
		border-bottom: 1px solid #f3f4f6;
	}

	.detail-row:last-child {
		border-bottom: none;
	}

	.label {
		font-weight: 600;
		color: #374151;
	}

	.value {
		color: #6b7280;
	}

	.blocked-count {
		font-weight: bold;
		color: #dc2626;
		font-size: 1.1em;
	}

	.statistics-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 20px;
		margin-bottom: 30px;
	}

	.stat-card {
		background: white;
		padding: 20px;
		border-radius: 8px;
		border: 1px solid #e5e7eb;
		text-align: center;
	}

	.stat-card h4 {
		margin: 0 0 10px 0;
		color: #6b7280;
		font-size: 0.9em;
	}

	.stat-value {
		font-size: 2em;
		font-weight: bold;
		color: #111827;
		margin-bottom: 5px;
	}

	.stat-label {
		color: #9ca3af;
		font-size: 0.8em;
	}

	.control-section {
		background: #f9fafb;
		padding: 24px;
		border-radius: 8px;
		margin-bottom: 30px;
	}

	.control-section h3 {
		margin: 0 0 15px 0;
		color: #374151;
	}

	.control-buttons {
		margin-bottom: 15px;
	}

	.btn {
		padding: 12px 24px;
		border: none;
		border-radius: 6px;
		cursor: pointer;
		font-weight: 600;
		font-size: 1em;
		transition: all 0.2s;
		display: inline-flex;
		align-items: center;
		gap: 8px;
	}

	.btn:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.btn-danger {
		background: #dc2626;
		color: white;
	}

	.btn-danger:hover:not(:disabled) {
		background: #b91c1c;
		transform: translateY(-1px);
	}

	.btn-success {
		background: #16a34a;
		color: white;
	}

	.btn-success:hover:not(:disabled) {
		background: #15803d;
		transform: translateY(-1px);
	}

	.btn-secondary {
		background: #6b7280;
		color: white;
	}

	.btn-secondary:hover:not(:disabled) {
		background: #4b5563;
	}

	.control-warning {
		background: #fef3c7;
		border: 1px solid #f59e0b;
		padding: 12px 16px;
		border-radius: 6px;
		color: #92400e;
		font-size: 0.9em;
	}

	.authorized-users {
		background: #eff6ff;
		padding: 20px;
		border-radius: 8px;
		border: 1px solid #bfdbfe;
	}

	.authorized-users h3 {
		margin: 0 0 15px 0;
		color: #1e40af;
	}

	.user-list {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		margin-bottom: 10px;
	}

	.user-badge {
		background: #2563eb;
		color: white;
		padding: 4px 12px;
		border-radius: 16px;
		font-size: 0.9em;
		font-weight: 500;
	}

	.user-note {
		color: #1e40af;
		font-size: 0.9em;
		margin: 0;
		font-style: italic;
	}

	.modal-backdrop {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal {
		background: white;
		border-radius: 12px;
		max-width: 500px;
		width: 90%;
		max-height: 80vh;
		overflow: auto;
		box-shadow: 0 20px 25px rgba(0, 0, 0, 0.3);
	}

	.emergency-modal {
		border: 3px solid #dc2626;
	}

	.normal-modal {
		border: 3px solid #16a34a;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 20px;
		border-bottom: 1px solid #e5e7eb;
	}

	.modal-header h3 {
		margin: 0;
		color: #1f2937;
	}

	.close-btn {
		background: none;
		border: none;
		font-size: 18px;
		cursor: pointer;
		padding: 5px;
		border-radius: 4px;
	}

	.close-btn:hover {
		background: #f3f4f6;
	}

	.modal-content {
		padding: 20px;
	}

	.warning-box {
		background: #fef2f2;
		border: 1px solid #fecaca;
		padding: 15px;
		border-radius: 8px;
		margin-bottom: 20px;
		color: #dc2626;
	}

	.info-box {
		background: #f0fdf4;
		border: 1px solid #bbf7d0;
		padding: 15px;
		border-radius: 8px;
		margin-bottom: 20px;
		color: #16a34a;
	}

	.confirmation-section {
		margin-bottom: 20px;
	}

	.confirmation-section label {
		display: block;
		margin-bottom: 8px;
		font-weight: 600;
		color: #374151;
	}

	.confirmation-input {
		width: 100%;
		padding: 12px;
		border: 2px solid #d1d5db;
		border-radius: 6px;
		font-size: 1em;
		font-family: monospace;
		text-transform: uppercase;
	}

	.confirmation-input:focus {
		outline: none;
		border-color: #2563eb;
	}

	.error-message {
		background: #fef2f2;
		color: #dc2626;
		padding: 10px;
		border-radius: 6px;
		margin-bottom: 15px;
		font-weight: 500;
	}

	.modal-actions {
		display: flex;
		gap: 10px;
		justify-content: flex-end;
	}

	/* Responsive design */
	@media (max-width: 768px) {
		.emergency-controls {
			padding: 15px;
		}

		.status-header {
			flex-direction: column;
			gap: 10px;
			text-align: center;
		}

		.detail-row {
			flex-direction: column;
			align-items: flex-start;
			gap: 5px;
		}

		.statistics-grid {
			grid-template-columns: 1fr;
		}

		.modal-actions {
			flex-direction: column;
		}

		.btn {
			width: 100%;
			justify-content: center;
		}
	}
</style>
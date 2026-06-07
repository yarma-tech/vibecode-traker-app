import Foundation
import SwiftData

/// Snapshot of a known file's sync state, passed into background parsing.
struct FileStateSnapshot: Sendable {
    let size: Int
    let modified: Date
}

/// A parsed `.jsonl` file plus the filesystem metadata to persist for it.
struct ParsedFile: Sendable {
    let path: String
    let size: Int
    let modified: Date
    let sessions: [ParsedSession]
}

/// All freshly parsed data for one project folder.
struct ProjectSessions: Sendable {
    let claudeProjectHash: String
    let cwd: String?
    let files: [ParsedFile]
    let latestActivity: Date
}

struct SessionSyncSummary: Equatable, Sendable {
    var sessionsCreated = 0
    var sessionsUpdated = 0
    var filesParsed = 0
    var filesSkipped = 0
}

/// Parses session transcripts for every detected project and upserts `Session`
/// rows. Filesystem reading + JSON parsing happen off the main actor; the
/// SwiftData upsert runs on the main actor.
final class SessionSyncService {
    let projectsDirectory: URL
    private let fileManager: FileManager
    private let parser = JSONLParser()

    init(projectsDirectory: URL = ClaudeProjectsScanner.defaultProjectsDirectory,
         fileManager: FileManager = .default) {
        self.projectsDirectory = projectsDirectory
        self.fileManager = fileManager
    }

    // MARK: - Parsing (background-safe)

    /// Parse every project folder, skipping `.jsonl` files unchanged since the
    /// last scan (per `knownStates`, keyed by file path).
    func parseAll(knownStates: [String: FileStateSnapshot]) -> [ProjectSessions] {
        guard let folders = try? fileManager.contentsOfDirectory(
            at: projectsDirectory,
            includingPropertiesForKeys: [.isDirectoryKey],
            options: [.skipsHiddenFiles]
        ) else { return [] }

        var result: [ProjectSessions] = []
        for folder in folders {
            let isDir = (try? folder.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) ?? false
            guard isDir else { continue }
            result.append(parseFolder(folder, knownStates: knownStates))
        }
        return result
    }

    private func parseFolder(_ folder: URL, knownStates: [String: FileStateSnapshot]) -> ProjectSessions {
        let hash = folder.lastPathComponent
        let keys: [URLResourceKey] = [.fileSizeKey, .contentModificationDateKey]
        let entries = (try? fileManager.contentsOfDirectory(
            at: folder,
            includingPropertiesForKeys: keys,
            options: [.skipsHiddenFiles]
        )) ?? []

        var parsedFiles: [ParsedFile] = []
        var cwd: String?
        var latest = Date(timeIntervalSince1970: 0)

        for file in entries where file.pathExtension == "jsonl" {
            let values = try? file.resourceValues(forKeys: Set(keys))
            let size = values?.fileSize ?? 0
            let modified = values?.contentModificationDate ?? Date(timeIntervalSince1970: 0)
            if modified > latest { latest = modified }

            if let known = knownStates[file.path],
               known.size == size,
               abs(known.modified.timeIntervalSince(modified)) < 1 {
                continue // unchanged since last scan
            }

            let sessions = (try? parser.parseFile(at: file, defaultDate: modified)) ?? []
            if cwd == nil { cwd = sessions.compactMap(\.cwd).first }
            parsedFiles.append(ParsedFile(path: file.path, size: size, modified: modified, sessions: sessions))
        }

        return ProjectSessions(claudeProjectHash: hash, cwd: cwd, files: parsedFiles, latestActivity: latest)
    }

    // MARK: - Upsert (main actor)

    @MainActor
    @discardableResult
    func apply(_ projectSessions: [ProjectSessions], into context: ModelContext) throws -> SessionSyncSummary {
        var projectsByHash: [String: Project] = [:]
        for project in try context.fetch(FetchDescriptor<Project>()) {
            projectsByHash[project.claudeProjectHash] = project
        }
        var sessionsById: [String: Session] = [:]
        for session in try context.fetch(FetchDescriptor<Session>()) {
            sessionsById[session.sessionId] = session
        }
        var statesByPath: [String: FileSyncState] = [:]
        for state in try context.fetch(FetchDescriptor<FileSyncState>()) {
            statesByPath[state.path] = state
        }

        var summary = SessionSyncSummary()

        for projectData in projectSessions {
            let project = projectsByHash[projectData.claudeProjectHash] ?? makeProject(for: projectData, in: context, index: &projectsByHash)

            if let cwd = projectData.cwd, !cwd.isEmpty {
                project.path = cwd
                project.name = ClaudeProjectsScanner.displayName(forPath: cwd)
                project.pathIsProvisional = false
            }
            if projectData.latestActivity > project.lastActivityAt {
                project.lastActivityAt = projectData.latestActivity
            }

            for file in projectData.files {
                summary.filesParsed += 1
                for parsed in file.sessions {
                    if let existing = sessionsById[parsed.sessionId] {
                        update(existing, from: parsed, project: project)
                        summary.sessionsUpdated += 1
                    } else {
                        let session = Session(sessionId: parsed.sessionId, startedAt: parsed.startedAt)
                        update(session, from: parsed, project: project)
                        context.insert(session)
                        sessionsById[parsed.sessionId] = session
                        summary.sessionsCreated += 1
                    }
                }
                upsertFileState(file, in: context, index: &statesByPath)
            }
        }

        if context.hasChanges { try context.save() }
        return summary
    }

    /// Convenience: fetch known states (main), parse (background), upsert (main).
    @MainActor
    @discardableResult
    func sync(into context: ModelContext) async throws -> SessionSyncSummary {
        var known: [String: FileStateSnapshot] = [:]
        for state in try context.fetch(FetchDescriptor<FileSyncState>()) {
            known[state.path] = FileStateSnapshot(size: state.lastOffset, modified: state.lastModified)
        }
        let parsed = await Task.detached(priority: .utility) { [self] in
            parseAll(knownStates: known)
        }.value
        let summary = try apply(parsed, into: context)
        Log.parser.info("Session sync: \(summary.sessionsCreated) new, \(summary.sessionsUpdated) updated, \(summary.filesParsed) files parsed.")
        return summary
    }

    // MARK: - Helpers

    @MainActor
    private func makeProject(for data: ProjectSessions, in context: ModelContext, index: inout [String: Project]) -> Project {
        let path = data.cwd ?? ClaudeProjectsScanner.decodePath(fromHash: data.claudeProjectHash)
        let project = Project(
            name: ClaudeProjectsScanner.displayName(forPath: path),
            path: path,
            claudeProjectHash: data.claudeProjectHash,
            firstSeenAt: data.latestActivity,
            lastActivityAt: data.latestActivity,
            pathIsProvisional: data.cwd == nil
        )
        context.insert(project)
        index[data.claudeProjectHash] = project
        return project
    }

    @MainActor
    private func update(_ session: Session, from parsed: ParsedSession, project: Project) {
        session.startedAt = parsed.startedAt
        session.endedAt = parsed.endedAt
        session.durationSeconds = parsed.durationSeconds
        session.model = parsed.model
        session.modelFamily = parsed.modelFamily
        session.inputTokens = parsed.inputTokens
        session.outputTokens = parsed.outputTokens
        session.cacheReadTokens = parsed.cacheReadTokens
        session.cacheCreationTokens = parsed.cacheCreationTokens
        session.messageCount = parsed.messageCount
        session.firstUserPrompt = parsed.firstUserPrompt
        session.project = project
    }

    @MainActor
    private func upsertFileState(_ file: ParsedFile, in context: ModelContext, index: inout [String: FileSyncState]) {
        if let state = index[file.path] {
            state.lastOffset = file.size
            state.lastModified = file.modified
        } else {
            let state = FileSyncState(path: file.path, lastOffset: file.size, lastModified: file.modified)
            context.insert(state)
            index[file.path] = state
        }
    }
}

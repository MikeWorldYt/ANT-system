import { AfterViewInit, Component, ElementRef, OnInit, QueryList, ViewChildren } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { CommonModule } from '@angular/common';
import { Subscription } from 'rxjs';

// ▲ COMPONENTS ▲
import { FooterComponent } from '../../../layout/footer/footer.component';

// ▲ CONTENT ▲
import { content } from '../../content/content';

// ▲ SERVICES ▲
import { ArticleService } from '../../../services/navArticleObserver.service';
import { LanguageService } from '../../../services/navLanguage.service';
import { TitleService } from '../../../services/navTitle.service';
import { PageService } from '../../../services/navPage.service';
import { HashHoverFeature } from '../../../services/ftHashHover.service';

@Component({
  selector: 'docs-t01-into-getting-starter',
  standalone: true,
  imports: [
    RouterOutlet,
    FooterComponent,
    CommonModule
  ],
  templateUrl: './into-getting-starter.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T01_IntoComponent implements OnInit, AfterViewInit {

  constructor(
    private languageService: LanguageService,
    private TitleService: TitleService,
    private pageService: PageService,
    private intersectionService: ArticleService,
    public hashHoverFeature: HashHoverFeature,
  ) { }

  // For inner content
  write: any;

  // Language
  currentLanguage: string = 'ES';
  currentTitle: string = 'title_01';
  currentPage: string = 'page_01';
  currentArticle: string = '';

  // ███ OnInit ███
  ngOnInit(): void {
    this.subscribeToLanguageChanges();
    this.TitleService.setTitle('title_01');
    this.pageService.setCurrentPage('page_01');
    this.initializeContent(this.currentLanguage, this.currentTitle, this.currentPage);
  }

  private initializeContent(language: string, title: string, page: string): void {
    this.write = content[language][title][page];
  }

  languageSubscription: Subscription = new Subscription();
  private subscribeToLanguageChanges(): void {
    this.languageSubscription = this.languageService.currentLanguage$.subscribe((language: string) => {
      this.currentLanguage = language as string;
      this.initializeContent(this.currentLanguage, this.currentTitle, this.currentPage); 
    });
  }

  // DOM QueryList Sections
  @ViewChildren('section') sections!: QueryList<ElementRef>;

  // ████ AfterViewInit ████
  ngAfterViewInit() {
    this.sections.forEach(section => {
      this.intersectionService.observe(section.nativeElement);
  });
  }

}
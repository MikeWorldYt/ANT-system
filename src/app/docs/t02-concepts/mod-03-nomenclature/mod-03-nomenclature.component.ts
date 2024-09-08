import { CommonModule } from '@angular/common';
import { AfterViewInit, Component, ElementRef, OnInit, QueryList, ViewChildren } from '@angular/core';
import { Subscription } from 'rxjs';

// ▲ SERVICES ▲
import { ArticleService } from '../../../services/navArticleObserver.service';
import { LanguageService } from '../../../services/navLanguage.service';
import { Language } from '../../../services/language.types';

// ▲ CONTENT ▲
import { content } from '../../content/content';
import { PageService } from '../../../services/navPage.service';
import { TitleService } from '../../../services/navTitle.service';

@Component({
  selector: 'doc-t02-mod-03-nomenclature',
  standalone: true,
  imports: [
    CommonModule
  ],
  templateUrl: './mod-03-nomenclature.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_Mod03_Component implements OnInit, AfterViewInit {
  // ▲ SERVICES ▲
  constructor(
    private languageService: LanguageService,
    private titleService: TitleService,
    private pageService: PageService,
    private intersectionService: ArticleService,
  ) { }

  // ▬▬▬ For inner content
  write: any;

  // ▬▬▬ Navigation Context
  currentLanguage: Language = 'EN';
  currentTitle: string = 'title_02';
  currentPage: string = 'page_03';
  currentArticle: string = '';

  // ███ OnInit ███ 
  ngOnInit(): void {
    this.subscribeToLanguageChanges();
    this.pageService.setCurrentPage('page_03');
    this.initializeContent(this.currentLanguage, this.currentTitle, this.currentPage);
  }

  private initializeContent(language: Language, title: string, page: string): void {
    this.write = content[language][title][page];
  }

  languageSubscription: Subscription = new Subscription();
  private subscribeToLanguageChanges(): void {
    this.languageSubscription = this.languageService.currentLanguage$.subscribe((language: string) => {
      this.currentLanguage = language as Language;
      this.initializeContent(this.currentLanguage, this.currentTitle, this.currentPage); 
    });
  }

  // ███ Hash Sections 
  hovered = false;
  showHash(event: Event) {
    this.hovered = true;
  }

  hideHash(event: Event) {
    this.hovered = false;
  }

  // ▲ DOM QueryList Sections
  @ViewChildren('section') sections!: QueryList<ElementRef>;

  // ████ AfterViewInit ████
  ngAfterViewInit() {
    // Intersection Section
    this.sections.forEach(section => {
      this.intersectionService.observe(section.nativeElement);
    });
  }

}
